import { readFileSync } from 'node:fs';
import { resolve, extname } from 'node:path';
import { TEMPLATES, interfaceText } from './templates.mjs';

export const escapeHTML = text => String(text).replace(/[&<>"']/g, char => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[char]);

const SVG_ELEMENTS = new Set('svg g defs title desc path rect circle ellipse line polyline polygon text tspan marker linearGradient radialGradient stop clipPath use'.split(' '));
const SVG_ATTRIBUTES = new Set(('id class role aria-label aria-labelledby aria-describedby xmlns xmlns:xlink viewBox width height x y x1 x2 y1 y2 cx cy r rx ry d points transform fill fill-opacity fill-rule stroke stroke-width stroke-opacity stroke-linecap stroke-linejoin stroke-dasharray stroke-dashoffset stroke-miterlimit opacity color font-family font-size font-weight font-style text-anchor dominant-baseline alignment-baseline dx dy textLength lengthAdjust letter-spacing word-spacing marker-start marker-mid marker-end markerWidth markerHeight refX refY orient markerUnits gradientUnits gradientTransform offset stop-color stop-opacity fx fy fr spreadMethod clip-path clip-rule clipPathUnits mask filter href xlink:href preserveAspectRatio vector-effect').split(' '));

function inlineSVG(source, title, prefix, report) {
  let failed = false;
  const bad = (at, tag, problem) => {
    failed = true;
    report(`SVG L${source.slice(0, at).split('\n').length} ${tag}：${problem}`);
  };
  function decode(value, at, tag) {
    return value.replace(/&([^;\s]*);?/g, (whole, entity) => {
      const named = { amp: '&', lt: '<', gt: '>', quot: '"', apos: "'" };
      if (whole.endsWith(';') && Object.hasOwn(named, entity)) return named[entity];
      if (whole.endsWith(';') && /^#(?:[0-9]+|x[0-9a-f]+)$/i.test(entity)) {
        const code = entity[1].toLowerCase() === 'x' ? parseInt(entity.slice(2), 16) : +entity.slice(1);
        if (code > 0 && code <= 0x10ffff && !(code >= 0xd800 && code <= 0xdfff)) return String.fromCodePoint(code);
      }
      bad(at, tag, '无效 XML 实体');
      return '';
    });
  }
  const tokens = /<!--[\s\S]*?-->|<\?xml\s[\s\S]*?\?>|<[^>]*>|[^<]+/g;
  const stack = [];
  let output = '', roots = 0, end = 0;
  for (const match of source.matchAll(tokens)) {
    const token = match[0], at = match.index;
    if (at !== end) bad(end, '<', '标记不完整');
    end = at + token.length;
    if (token.startsWith('<!--') || token.startsWith('<?xml ')) continue;
    if (!token.startsWith('<')) {
      if (!stack.length && token.trim()) bad(at, 'svg', '根元素外有文字');
      output += escapeHTML(decode(token, at, 'text'));
      continue;
    }
    const tag = token.match(/^<(\/)?([A-Za-z][\w:-]*)([\s\S]*?)(\/?)>$/);
    if (!tag) { bad(at, token.slice(0, 24), '不允许声明或无效标签'); continue; }
    const [, closing, name, raw, selfClosing] = tag;
    if (!SVG_ELEMENTS.has(name)) { bad(at, name, '元素不在白名单'); continue; }
    if (closing) {
      if (raw.trim() || selfClosing || stack.pop() !== name) bad(at, name, '闭合标签不匹配');
      output += `</${name}>`;
      continue;
    }
    const attrs = new Map();
    let consumed = 0;
    const pattern = /\s+([A-Za-z_:][\w:.-]*)\s*=\s*("[^"]*"|'[^']*')/g;
    for (const attribute of raw.matchAll(pattern)) {
      if (attribute.index !== consumed) bad(at, name, '属性语法不完整');
      consumed = attribute.index + attribute[0].length;
      const key = attribute[1], value = decode(attribute[2].slice(1, -1), at, name);
      if (!SVG_ATTRIBUTES.has(key) || /^on/i.test(key) || key === 'style') bad(at, name, `属性 ${key} 不允许`);
      if (attrs.has(key)) bad(at, name, `属性 ${key} 重复`);
      if ((key === 'xmlns' && value !== 'http://www.w3.org/2000/svg') || (key === 'xmlns:xlink' && value !== 'http://www.w3.org/1999/xlink')) bad(at, name, '命名空间不允许');
      if (['href', 'xlink:href'].includes(key) && !/^#[^\s]+$/.test(value)) bad(at, name, 'href 只允许 #片段');
      if (/url\s*\(/i.test(value)) {
        const rest = value.replace(/url\(\s*(['"]?)#[\w:.-]+\1\s*\)/gi, '');
        if (rest.trim()) bad(at, name, 'url() 只允许本文件 #片段');
      }
      if (/[\\<>]/.test(value)) bad(at, name, '属性含不支持的转义或标记');
      // Inlined figures share the page's ID space; unprefixed, a #reference resolves to another figure or a page anchor.
      attrs.set(key, key === 'id' ? prefix + value
        : ['href', 'xlink:href'].includes(key) ? value.replace(/^#/, `#${prefix}`)
        : ['aria-labelledby', 'aria-describedby'].includes(key) ? value.replace(/\S+/g, id => prefix + id)
        : value.replace(/url\(\s*(['"]?)#([\w:.-]+)\1\s*\)/gi, (whole, quote, id) => `url(#${prefix}${id})`));
    }
    if (raw.slice(consumed).trim()) bad(at, name, '属性须为带引号的键值');
    if (!stack.length) {
      roots++;
      if (name !== 'svg' || roots !== 1) bad(at, name, '必须只有一个 svg 根元素');
      if (!attrs.has('viewBox')) bad(at, name, '根元素缺少 viewBox');
      attrs.set('role', 'img'); attrs.set('aria-label', title);
    }
    output += `<${name}${[...attrs].map(([key, value]) => ` ${key}="${escapeHTML(value)}"`).join('')}${selfClosing ? '/>' : '>'}`;
    if (!selfClosing) stack.push(name);
  }
  if (end !== source.length || stack.length || roots !== 1) bad(end, 'svg', '根元素缺失或标签未闭合');
  return failed ? '' : output;
}

export function parseDraft(text, { cwd, assets = null }) {
  const snapshot = Object.create(null);
  let references = [];
  const rawLines = text.match(/[^\r\n]*(?:\r\n|\r|\n|$)/g);
  function resource(file) {
    references.push(file);
    if (!Object.hasOwn(snapshot, file)) {
      if (assets && Object.hasOwn(assets, file)) snapshot[file] = assets[file];
      else snapshot[file] = readFileSync(resolve(cwd, file)).toString('base64');
    }
    return snapshot[file] === null ? null : Buffer.from(snapshot[file], 'base64');
  }
  const lines = text.replace(/\r\n?/g, '\n').split('\n');
  const errors = [];
  const warnings = [];
  const meta = { source: [] };
  const error = (line, problem, fix) => errors.push({ line, message: `L${line}: ${problem}。改法：${fix}` });
  let pos = 0;
  if (lines[0] !== '---') error(1, '缺少 frontmatter', '用 --- 包住 template、title、lang');
  else {
    pos = 1;
    for (; pos < lines.length && lines[pos] !== '---'; pos++) {
      const match = lines[pos].match(/^([a-z]+):\s*(.*)$/);
      if (!match || !['template', 'title', 'lang', 'subtitle', 'source', 'progress'].includes(match[1])) {
        error(pos + 1, '未知或无效的 frontmatter 键', '使用 template、title、lang、subtitle、source 或 progress');
        continue;
      }
      const [, key, value] = match;
      if (!value.trim()) error(pos + 1, `${key} 为空`, '填写非空值');
      if (key === 'source') meta.source.push(value.trim());
      else if (Object.hasOwn(meta, key)) error(pos + 1, `${key} 重复`, '只保留一行');
      else meta[key] = value.trim();
    }
    if (pos === lines.length) error(1, 'frontmatter 未闭合', '在正文前添加 ---');
    else pos++;
  }
  for (const key of ['template', 'title', 'lang']) if (!meta[key]) error(1, `缺少 ${key}`, `在 frontmatter 填写 ${key}`);
  if (meta.template && !Object.hasOwn(TEMPLATES, meta.template)) error(1, 'template 无效', '使用 general、prd、crd 或 plan');
  if (meta.template === 'plan' && !['todo', 'partial', 'done'].includes(meta.progress)) error(1, 'plan 的 progress 无效或缺失', '填写 todo、partial 或 done');
  if (meta.template !== 'plan' && Object.hasOwn(meta, 'progress')) error(1, '只有 plan 可用 progress', '删除 progress');
  const ui = interfaceText(meta.lang || 'en');

  function inline(value, line) {
    const pattern = /`([^`]+)`|\*\*([^*]+)\*\*|\[([^\]]+)\]\(([^)]*)\)/g;
    let result = '', end = 0;
    for (const match of value.matchAll(pattern)) {
      result += escapeHTML(value.slice(end, match.index));
      if (match[1] !== undefined) result += `<code>${escapeHTML(match[1])}</code>`;
      else if (match[2] !== undefined) result += `<strong>${inline(match[2], line)}</strong>`;
      else {
        const href = match[4];
        if (!/^(https?:\/\/|mailto:|#)/.test(href)) error(line, `链接协议不允许：${href}`, '使用 http://、https://、mailto: 或 # 锚点');
        result += `<a href="${escapeHTML(href)}">${escapeHTML(match[3])}</a>`;
      }
      end = match.index + match[0].length;
    }
    return result + escapeHTML(value.slice(end));
  }

  const special = value => /^(?:#{1,6}\s|\s*-\s|\d+\.\s|\||```|:::|>|!\[)/.test(value);
  const cells = value => value.trim().replace(/^\|/, '').replace(/\|$/, '').split('|').map(cell => cell.trim());

  let tableNumber = 0, svgNumber = 0;
  function blocks(rows, onlyParagraphs = false, simple = false) {
    const result = [];
    for (let i = 0; i < rows.length;) {
      const { value, line } = rows[i];
      if (!value.trim()) { i++; continue; }
      if (onlyParagraphs && special(value)) error(line, '引言只能包含段落', '把此块移到第一个 ## 小节后');
      if (simple && special(value) && !/^\s*-\s/.test(value)) error(line, '此容器只能包含段落与列表', '把其他块移到容器外');
      if (value.startsWith('```')) {
        const start = i++;
        while (i < rows.length && rows[i].value !== '```') i++;
        if (i === rows.length) error(line, '代码块未闭合', '添加单独一行 ```');
        result.push(`<pre><code>${escapeHTML(rows.slice(start + 1, i).map(row => row.value).join('\n'))}</code></pre>`);
        i++;
      } else if (/^###\s+/.test(value)) {
        result.push(`<h3>${inline(value.slice(4), line)}</h3>`); i++;
      } else if (/^#+\s/.test(value)) {
        error(line, '不支持此级标题', '小节用 ## slot | 标题，小节内用 ### 标题'); i++;
      } else if (/^\s*-\s/.test(value)) {
        let html = '<ul>', nested = false, item = false;
        while (i < rows.length && /^\s*-\s/.test(rows[i].value)) {
          const row = rows[i++];
          const match = row.value.match(/^(\s*)-\s+(.*)$/);
          const depth = match[1].length;
          if (![0, 2].includes(depth)) error(row.line, '列表最多两层，子项缩进两个空格', '拆成两个列表或改用表格');
          if (depth && !item) error(row.line, '子项缺少父项', '先写不缩进的父项');
          if (depth) {
            if (!nested) { html += '<ul>'; nested = true; }
            html += `<li>${inline(match[2], row.line)}</li>`;
          } else {
            if (nested) { html += '</ul>'; nested = false; }
            if (item) html += '</li>';
            html += `<li>${inline(match[2], row.line)}`;
            item = true;
          }
        }
        result.push(html + (nested ? '</ul>' : '') + (item ? '</li>' : '') + '</ul>');
      } else if (/^\d+\.\s/.test(value)) {
        let html = '<ol class="steps">', number = 1;
        while (i < rows.length && /^\d+\.\s/.test(rows[i].value)) {
          const row = rows[i++], match = row.value.match(/^(\d+)\.\s+(.*)$/);
          if (+match[1] !== number) error(row.line, '步骤序号不连续', `此项改为 ${number}.`);
          number++;
          html += `<li>${inline(match[2], row.line)}</li>`;
        }
        result.push(html + '</ol>');
      } else if (value.startsWith('|')) {
        const header = cells(value);
        const separator = rows[i + 1];
        if (!separator || cells(separator.value).length !== header.length || !cells(separator.value).every(cell => /^:?-{3,}:?$/.test(cell))) {
          error(line, '表格缺少有效分隔行', '第二行每列填写 ---，列数与表头相同');
          i++;
          continue;
        }
        i += 2;
        const notes = [];
        const heads = header.map(cell => {
          const split = cell.indexOf('::');
          if (split < 0) return `<th scope="col">${inline(cell, line)}</th>`;
          const label = cell.slice(0, split).trim(), meaning = cell.slice(split + 2).trim();
          if (!meaning) error(line, '表头释义为空', '在 :: 后解释该列的含义');
          notes.push(`<dt>${inline(label, line)}</dt><dd>${inline(meaning, line)}</dd>`);
          return `<th scope="col">${inline(label, line)}</th>`;
        });
        let html = `<div class="table-scroll" role="region" tabindex="0" aria-label="${ui.table}"><table><thead><tr>${heads.join('')}</tr></thead><tbody>`;
        while (i < rows.length && rows[i].value.startsWith('|')) {
          const row = rows[i++], values = cells(row.value);
          if (values.length !== header.length) error(row.line, `表格应为 ${header.length} 列，实际 ${values.length} 列`, '删除多余列或补齐缺失列');
          html += `<tr>${values.map(cell => {
            const status = cell.match(/^\[(ok|no|warn)\](?:\s+(.*))?$/);
            if (!status) return `<td>${inline(cell, row.line)}</td>`;
            if (!status[2]?.trim()) error(row.line, '状态标记后没有文字', '在标记后填写状态含义');
            return `<td><span class="status ${status[1]}"><span aria-hidden="true">${{ ok: '✓', no: '✗', warn: '!' }[status[1]]}</span> ${inline(status[2] || '', row.line)}</span></td>`;
          }).join('')}</tr>`;
        }
        html += '</tbody></table></div>';
        if (notes.length) {
          const id = `notes-${++tableNumber}`;
          html += `<div class="column-notes"><button type="button" class="notes-toggle" aria-expanded="true" aria-controls="${id}" hidden>${ui.notes}</button><dl id="${id}">${notes.join('')}</dl></div>`;
        }
        result.push(html);
      } else if (value.startsWith('![')) {
        const image = value.match(/^!\[([^\]]*)\]\((.+)\)$/);
        if (!image) error(line, '图片格式不正确', '单独一行写 ![说明](本地路径)');
        else {
          const [, caption, file] = image;
          const extension = extname(file).slice(1).toLowerCase();
          if (/^[a-z][a-z0-9+.-]*:/i.test(file) || file.startsWith('//')) error(line, '图片必须是本地文件', '先存成本地文件再引用');
          else if (!['png', 'jpg', 'jpeg', 'gif', 'webp', 'avif', 'svg'].includes(extension)) error(line, '不支持此图片扩展名', '使用 png、jpg、jpeg、gif、webp、avif 或 svg');
          else {
            let bytes;
            try { bytes = resource(file); }
            catch { snapshot[file] = null; }
            if (!bytes) warnings.push(`WARN L${line}: ${ui.missing}：${file}`);
            const mime = { jpg: 'jpeg', svg: 'svg+xml' }[extension] || extension;
            const ref = caption.trim() ? '' : ` data-ref="${escapeHTML(file)}"`;
            result.push(`<figure${ref}>${bytes ? `<div class="image-scroll" role="region" tabindex="0" aria-label="${escapeHTML(caption)}"><img src="data:image/${mime};base64,${bytes.toString('base64')}" alt="${escapeHTML(caption)}"></div>` : `<div class="missing-image">${ui.missing}：${escapeHTML(file)}</div>`}<figcaption>${inline(caption, line)}</figcaption></figure>`);
          }
        }
        i++;
      } else if (value.startsWith(':::')) {
        const start = i++, container = value.match(/^:::(conclusion|reminder|metrics|figure)(?:\s+(.*))?$/);
        while (i < rows.length && rows[i].value !== ':::') i++;
        if (i === rows.length) error(line, '容器未闭合', '添加单独一行 :::');
        const content = rows.slice(start + 1, i).filter(row => row.value.trim());
        i++;
        if (!container) { error(line, '未知容器', '使用 conclusion、reminder、metrics 或 figure'); continue; }
        const [, kind, title = ''] = container;
        if (kind === 'metrics') {
          if (!content.length) error(line, '指标块没有内容', '每行填写数值 | 名称 | 含义');
          result.push(`<div class="metrics">${content.map(row => {
            const parts = row.value.split('|').map(part => part.trim());
            if (parts.length !== 3 || parts.some(part => !part)) error(row.line, '指标须有非空的三段', '填写数值 | 名称 | 含义');
            return `<div class="metric"><p class="metric-value">${inline(parts[0] || '', row.line)}</p><p class="metric-name">${inline(parts[1] || '', row.line)}</p><p>${inline(parts[2] || '', row.line)}</p></div>`;
          }).join('')}</div>`);
        } else if (kind === 'figure') {
          if (!title.trim()) error(line, '图缺少标题', '在 :::figure 后填写标题');
          const sources = content.filter(row => /^(reader|svg):/.test(row.value));
          const source = content[0]?.value.match(/^(reader|svg):\s*(\S.*)$/);
          if (sources.length !== 1 || !source) { error(line, '图须恰好一行来源且放在图注之前', '首行写 reader: 路径.html 或 svg: 路径.svg，其后只写图注'); continue; }
          const [, type, file] = source;
          let data;
          try { data = resource(file).toString('utf8'); }
          catch { error(content[0].line, `无法读取图文件 ${file}`, '确认本地路径存在且可读'); continue; }
          // Host styles cannot hide a reader control across the iframe boundary when scripts are disabled.
          if (type === 'reader') data = data.replace(/<\/head>/i, '<noscript><style>#btn-theme{display:none!important}</style></noscript></head>');
          const graphic = type === 'reader'
            ? `<div class="reader-frame"><iframe srcdoc="${escapeHTML(data)}" title="${escapeHTML(title)}" loading="lazy"></iframe></div>`
            : inlineSVG(data, title, `fig${++svgNumber}-`, problem => error(content[0].line, problem, '按 SVG 元素、属性与本地片段白名单修正源文件'));
          result.push(`<figure class="diagram">${graphic}<figcaption><strong>${escapeHTML(title)}</strong>${content.slice(1).map(row => `<p>${inline(row.value, row.line)}</p>`).join('')}</figcaption></figure>`);
        } else {
          const body = blocks(rows.slice(start + 1, i - 1), false, true);
          result.push(`<aside class="${kind}">${title ? `<h3>${inline(title, line)}</h3>` : ''}${body.join('')}</aside>`);
        }
      } else if (value.startsWith('>')) {
        error(line, '不支持引用块', '改用 :::reminder 或普通段落'); i++;
      } else {
        const paragraph = [inline(value, line)];
        i++;
        while (i < rows.length && rows[i].value.trim() && !special(rows[i].value)) {
          paragraph.push(inline(rows[i].value, rows[i].line)); i++;
        }
        result.push(`<p>${paragraph.join('\n')}</p>`);
      }
    }
    return result;
  }

  const leadRows = [], sections = [];
  let current = leadRows, fenced = false, container = false;
  for (; pos < lines.length; pos++) {
    const value = lines[pos], line = pos + 1;
    if (!fenced && !container && /^##(?:\s|$)/.test(value)) {
      const match = value.match(/^##\s+([a-z]+)\s*\|\s*(\S.*)$/);
      const section = { slot: match?.[1], title: match?.[2] || '', line, rows: [] };
      if (!match) error(line, '小节标题格式不正确', '写成 ## slot | 标题');
      sections.push(section); current = section.rows;
    } else current.push({ value, line });
    if (value.startsWith('```')) fenced = !fenced;
    if (!fenced && value.startsWith(':::')) container = value !== ':::';
  }
  const lead = blocks(leadRows, true);
  const region = rows => ({ raw: rows.map(row => rawLines[row.line - 1]).join(''), references: [...new Set(references)] });
  const regions = { lead: region(leadRows), sections: [] };
  if (!lead.length) error(pos + 1, '缺少引言段落', '在第一个 ## 前写主题结论');
  const template = Object.hasOwn(TEMPLATES, meta.template) ? TEMPLATES[meta.template] : undefined;
  const spec = meta.template === 'plan' ? (Object.hasOwn(template, meta.progress) ? template[meta.progress] : undefined) : template;
  let previous = -1;
  const seen = new Set();
  for (const section of sections) {
    if (spec && section.slot) {
      const order = spec.slots.indexOf(section.slot);
      if (order < 0) error(section.line, `模板不支持 slot ${section.slot}`, `使用 ${spec.slots.join('、')}`);
      else if (seen.has(section.slot) && section.slot !== 'more') error(section.line, `slot ${section.slot} 重复`, '合并到同一个小节');
      else if (order < previous) error(section.line, '小节顺序不正确', `按 ${spec.slots.join(' → ')} 排列`);
      if (order >= 0) { previous = Math.max(previous, order); seen.add(section.slot); }
    }
    references = [];
    section.blocks = blocks(section.rows);
    regions.sections.push(region(section.rows));
    if (!section.blocks.length) error(section.line, '小节没有内容', '添加正文；原文未说明时如实写明');
    delete section.rows;
  }
  if (spec) for (const slot of spec.required) if (!seen.has(slot)) error(lines.length, `缺少必填 slot ${slot}`, `添加 ## ${slot} | 标题 及正文`);
  errors.sort((a, b) => a.line - b.line);
  return { meta, lead, sections, regions, assets: snapshot, errors: errors.map(item => item.message), warnings };
}
