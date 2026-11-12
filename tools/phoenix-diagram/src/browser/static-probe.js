// Measure the actual inline SVG; this probe does not assert or review pixels.
(async function () {
  'use strict';
  await (document.fonts ? document.fonts.ready : Promise.resolve());
  await new Promise(function (resolve) {
    requestAnimationFrame(function () { requestAnimationFrame(resolve); });
  });

  var html = document.documentElement;
  var figures = Array.from(document.querySelectorAll('svg[data-phoenix-static]'));
  var svg = figures[0] || null;
  var rect = function (element) {
    if (!element) return null;
    var value = element.getBoundingClientRect();
    return { x: value.x, y: value.y, top: value.top, right: value.right, bottom: value.bottom, left: value.left, width: value.width, height: value.height };
  };
  var finiteRect = function (value) {
    return Boolean(value && [value.x, value.y, value.width, value.height].every(Number.isFinite));
  };
  var visible = function (element) {
    if (!element || element.hidden || element.closest('[hidden]')) return false;
    var style = getComputedStyle(element);
    return style.display !== 'none' && style.visibility !== 'hidden' && Number(style.opacity) > 0 && element.getClientRects().length > 0;
  };
  var colorVisible = function (color) {
    return Boolean(color && color !== 'none' && color !== 'transparent' && !/^rgba\([^)]*,\s*0(?:\.0+)?\s*\)$/.test(color) && !/\/\s*0(?:\.0+)?\s*\)$/.test(color));
  };
  var paint = function (element, kind) {
    var bounds = rect(element), reasons = [], style = element && getComputedStyle(element);
    if (!element) reasons.push('missing');
    for (var ancestor = element; ancestor; ancestor = ancestor.parentElement) {
      var inherited = getComputedStyle(ancestor);
      if (ancestor.hidden || inherited.display === 'none' || ['hidden', 'collapse'].includes(inherited.visibility) || !(Number(inherited.opacity) > 0)) {
        reasons.push('hidden:' + ancestor.tagName.toLowerCase());
        break;
      }
    }
    if (!finiteRect(bounds) || !element || !element.getClientRects().length ||
        (kind === 'edge' ? !(bounds.width > 0 || bounds.height > 0) : !(bounds.width > 0 && bounds.height > 0))) reasons.push('empty-bounds');
    var fill = Boolean(style && colorVisible(style.fill) && Number(style.fillOpacity) > 0);
    var stroke = Boolean(style && colorVisible(style.stroke) && Number(style.strokeOpacity) > 0 && parseFloat(style.strokeWidth) > 0);
    if (kind === 'edge' ? !stroke : !fill && !stroke) reasons.push('no-paint');
    var length = null;
    if (kind === 'edge' && element) {
      try { length = element.getTotalLength(); } catch (_) {}
      if (!element.getAttribute('d') || !(Number.isFinite(length) && length > 0)) reasons.push('empty-path');
    }
    return { visible: reasons.length === 0, bounds: bounds, reasons: reasons, length: length, fill: style && style.fill, stroke: style && style.stroke };
  };
  var contains = function (outer, inner, tolerance) {
    return Boolean(outer && inner && inner.left >= outer.left - tolerance && inner.right <= outer.right + tolerance && inner.top >= outer.top - tolerance && inner.bottom <= outer.bottom + tolerance);
  };
  var overflowClips = function (element, bounds) {
    var failures = [];
    for (var ancestor = element && element.parentElement; ancestor; ancestor = ancestor.parentElement) {
      var style = getComputedStyle(ancestor), box = rect(ancestor);
      if (!box || style.display === 'contents') continue;
      var left = box.left + (parseFloat(style.borderLeftWidth) || 0);
      var right = box.right - (parseFloat(style.borderRightWidth) || 0);
      var top = box.top + (parseFloat(style.borderTopWidth) || 0);
      var bottom = box.bottom - (parseFloat(style.borderBottomWidth) || 0);
      var clips = function (overflow) { return ['hidden', 'clip', 'auto', 'scroll'].includes(overflow); };
      if (clips(style.overflowX) && (bounds.left < left - 1 || bounds.right > right + 1)) failures.push({ reason: 'ancestor-overflow-x', tag: ancestor.tagName, bounds: box });
      if (clips(style.overflowY) && (bounds.top < top - 1 || bounds.bottom > bottom + 1)) failures.push({ reason: 'ancestor-overflow-y', tag: ancestor.tagName, bounds: box });
    }
    return failures;
  };
  var elementFact = function (element) {
    return { tag: element.tagName.toLowerCase(), id: element.id || null, role: element.getAttribute('role'), bounds: rect(element), visible: visible(element) };
  };
  var externalReference = function (value) {
    if (!value || value.charAt(0) === '#') return false;
    try {
      var url = new URL(value, window.location.href);
      if (['data:', 'blob:'].includes(url.protocol)) return false;
      if (url.protocol === 'file:') {
        var documentUrl = new URL(window.location.href);
        url.search = ''; url.hash = ''; documentUrl.search = ''; documentUrl.hash = '';
        return url.href !== documentUrl.href;
      }
      return true;
    } catch (_) {
      return true;
    }
  };

  var viewBox = svg && svg.viewBox && svg.viewBox.baseVal;
  var bounds = rect(svg);
  var textElements = svg ? Array.from(svg.querySelectorAll('text')).filter(function (element) {
    return (element.textContent || '').trim();
  }) : [];
  var text = textElements.map(function (element) {
    var style = getComputedStyle(element), painted = paint(element, 'text');
    var sourceFontPx = parseFloat(style.fontSize);
    var matrix = element.getScreenCTM();
    var projectionScale = matrix ? Math.sqrt(matrix.a * matrix.a + matrix.b * matrix.b) : NaN;
    var node = element.closest('[data-node-id]');
    var edge = element.closest('[data-edge-from][data-edge-to]');
    return Object.assign(painted, {
      text: (element.textContent || '').trim(),
      sourcePath: element.getAttribute('data-source-path'),
      sourceFontPx: sourceFontPx,
      projectionScale: projectionScale,
      projectedFontPx: sourceFontPx * projectionScale,
      fontFamily: style.fontFamily,
      owner: node ? { kind: 'node', id: node.getAttribute('data-node-id') } : edge ? { kind: 'edge', id: edge.getAttribute('data-edge-id'), from: edge.getAttribute('data-edge-from'), to: edge.getAttribute('data-edge-to') } : null
    });
  });
  var minimum = text.reduce(function (best, entry) {
    return Number.isFinite(entry.projectedFontPx) && (!best || entry.projectedFontPx < best.projectedFontPx) ? entry : best;
  }, null);

  var nodeElements = svg ? Array.from(svg.querySelectorAll('[data-node-id]')) : [];
  var nodes = nodeElements.map(function (node) {
    var bodies = Array.from(node.querySelectorAll(':scope > [data-graph-role="node-body"]'));
    var labels = Array.from(node.querySelectorAll('text[data-node-label]')).filter(function (element) {
      return (element.textContent || '').trim();
    });
    var bodyFacts = bodies.map(function (body) { return paint(body, 'body'); });
    var labelFacts = labels.map(function (label) { return paint(label, 'text'); });
    return {
      id: node.getAttribute('data-node-id'),
      bounds: rect(node),
      bodies: bodyFacts,
      labels: labelFacts,
      visible: bodyFacts.length > 0 && labelFacts.length > 0 && bodyFacts.every(function (body) { return body.visible; }) && labelFacts.every(function (label) { return label.visible; })
    };
  });
  var edgeElements = svg ? Array.from(svg.querySelectorAll('path[data-graph-role="edge"]')) : [];
  var edges = edgeElements.map(function (edge) {
    return Object.assign({
      id: edge.getAttribute('data-edge-id'),
      key: edge.getAttribute('data-edge-key'),
      from: edge.getAttribute('data-edge-from'),
      to: edge.getAttribute('data-edge-to')
    }, paint(edge, 'edge'));
  });

  var nodeTextClips = [];
  textElements.forEach(function (element) {
    var node = element.closest('[data-node-id]');
    if (!node) return;
    var box = node.querySelector(':scope > rect.wf-row') || node.querySelector(':scope > [data-graph-role="node-body"]') || node.querySelector(':scope > rect:not(.c-mask)') || node.querySelector(':scope > rect');
    var textBounds = rect(element), boxBounds = rect(box);
    if (box && !contains(boxBounds, textBounds, 1)) nodeTextClips.push({ id: node.getAttribute('data-node-id'), text: (element.textContent || '').trim(), box: boxBounds, textBounds: textBounds });
  });
  var canvasTextChecks = textElements.map(function (element) {
    var textBounds = rect(element), failures = [];
    if (!contains(bounds, textBounds, 1)) failures.push({ reason: 'outside-svg' });
    failures = failures.concat(overflowClips(element, textBounds));
    return { text: (element.textContent || '').trim(), bounds: textBounds, failures: failures };
  });
  var canvasTextClips = canvasTextChecks.filter(function (entry) { return entry.failures.length > 0; });

  var cards = Array.from(document.querySelectorAll('.cards > .card, .cards > [data-card]')).map(function (element) {
    return Object.assign(elementFact(element), {
      scrollWidth: element.scrollWidth,
      clientWidth: element.clientWidth,
      scrollHeight: element.scrollHeight,
      clientHeight: element.clientHeight,
      clippedText: element.scrollWidth > element.clientWidth + 1 || element.scrollHeight > element.clientHeight + 1
    });
  });
  var viewerChrome = Array.from(document.querySelectorAll('.diagram-nav, .reader-rail, .reader-toolbar, .viewer-chrome, [data-viewer-chrome], .header, .node-outline, .cards')).map(elementFact);
  var controlElements = Array.from(document.querySelectorAll('button, input, select, textarea, a[href], [role="button"], [role="link"], [contenteditable], [data-view], [data-tree-toggle], [data-tree-expand-all]'));
  var controls = controlElements.map(function (element) {
    return Object.assign(elementFact(element), {
      disabled: Boolean(element.disabled),
      tabIndex: element.tabIndex,
      accessibleName: element.getAttribute('aria-label') || element.getAttribute('title') || (element.textContent || '').trim()
    });
  });
  var focusCandidates = Array.from(document.querySelectorAll('[tabindex], button, input, select, textarea, a[href], audio[controls], video[controls], [contenteditable]'));
  var tabStops = focusCandidates.filter(function (element) { return element.tabIndex >= 0; }).map(function (element) {
    return Object.assign(elementFact(element), { tabIndex: element.tabIndex });
  });
  var tabindexAttributes = Array.from(document.querySelectorAll('[tabindex]')).map(function (element) {
    return Object.assign(elementFact(element), { tabindex: element.getAttribute('tabindex') });
  });
  var interactiveSemantics = Array.from(document.querySelectorAll('[role="button"], [role="link"], [aria-pressed], [aria-expanded], [aria-controls], [contenteditable]')).map(function (element) {
    return Object.assign(elementFact(element), {
      ariaPressed: element.getAttribute('aria-pressed'),
      ariaExpanded: element.getAttribute('aria-expanded'),
      ariaControls: element.getAttribute('aria-controls')
    });
  });
  var eventHooks = [];
  Array.from(document.querySelectorAll('*')).forEach(function (element) {
    Array.from(element.attributes).forEach(function (attribute) {
      if (/^on/i.test(attribute.name) || ['data-animate', 'data-delta-animate', 'data-animation', 'data-tree-toggle', 'data-tree-expand-all', 'data-reader-legend-corner'].includes(attribute.name) || attribute.name.startsWith('data-motion-')) {
        eventHooks.push({ tag: element.tagName.toLowerCase(), id: element.id || null, attribute: attribute.name });
      }
    });
  });
  var forbiddenElements = Array.from(document.querySelectorAll('script, foreignObject, iframe, object, embed, audio, video')).map(elementFact);
  var externalReferences = [];
  Array.from(document.querySelectorAll('[href], [src], [xlink\\:href], [poster]')).forEach(function (element) {
    ['href', 'src', 'xlink:href', 'poster'].forEach(function (name) {
      var value = element.getAttribute(name);
      if (externalReference(value)) externalReferences.push({ tag: element.tagName.toLowerCase(), id: element.id || null, attribute: name, value: value });
    });
  });

  var fontFaces = [], fontFaceErrors = [];
  var readFontFaces = function (rules) {
    for (var rule of rules) {
      if (rule.type === CSSRule.FONT_FACE_RULE) {
        var source = rule.style.getPropertyValue('src');
        fontFaces.push({
          family: rule.style.getPropertyValue('font-family'),
          dataWoff2: /url\(\s*["']?data:font\/woff2;/i.test(source),
          local: /(?:^|[^a-z])local\s*\(/i.test(source)
        });
      } else if (rule.cssRules) {
        readFontFaces(rule.cssRules);
      }
    }
  };
  for (var sheet of document.styleSheets) {
    try { readFontFaces(sheet.cssRules); } catch (error) { fontFaceErrors.push({ message: String(error) }); }
  }
  var resources = performance.getEntriesByType('resource').map(function (entry) {
    return { name: entry.name, initiatorType: entry.initiatorType };
  });
  var fontResources = resources.filter(function (entry) { return /^data:font\//i.test(entry.name); });
  var backgroundElement = svg && svg.querySelector('.c-bg-rect');
  var background = backgroundElement ? getComputedStyle(backgroundElement).fill : null;
  var hostTheme = html.getAttribute('data-theme');
  var labelledName = '';
  if (svg) {
    var labelledBy = svg.getAttribute('aria-labelledby');
    if (labelledBy) labelledName = labelledBy.split(/\s+/).map(function (id) {
      var label = document.getElementById(id);
      return label ? (label.textContent || '').trim() : '';
    }).filter(Boolean).join(' ');
  }
  var title = svg && svg.querySelector(':scope > title');
  var accessibleName = svg ? (svg.getAttribute('aria-label') || labelledName || (title && title.textContent || '').trim()) : '';

  return {
    documentUrl: window.location.href,
    innerWidth: window.innerWidth,
    innerHeight: window.innerHeight,
    scrollWidth: Math.ceil(Math.max(html.scrollWidth, document.body.scrollWidth)),
    scrollHeight: Math.max(window.innerHeight, Math.ceil(Math.max(html.scrollHeight, document.body.scrollHeight))),
    scripts: document.scripts.length,
    figures: figures.length,
    bounds: bounds,
    viewBox: viewBox ? { x: viewBox.x, y: viewBox.y, width: viewBox.width, height: viewBox.height } : null,
    rootRole: svg && svg.getAttribute('role'),
    accessibleName: accessibleName,
    fontPolicy: svg && svg.getAttribute('data-font-policy'),
    hostTheme: hostTheme,
    resolvedTheme: hostTheme || (window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'),
    background: background,
    backgroundBounds: rect(backgroundElement),
    nodeCount: nodeElements.length,
    edgeCount: edgeElements.length,
    nodes: nodes,
    edges: edges,
    text: text,
    minimumProjectedTextPx: minimum && minimum.projectedFontPx,
    minimumProjectedText: minimum,
    nodeTextClips: nodeTextClips,
    canvasTextClips: canvasTextClips,
    canvasTextChecks: canvasTextChecks,
    cards: cards,
    viewerChrome: viewerChrome,
    controls: controls,
    tabStops: tabStops,
    tabindexAttributes: tabindexAttributes,
    interactiveSemantics: interactiveSemantics,
    eventHooks: eventHooks,
    forbiddenElements: forbiddenElements,
    fontsStatus: document.fonts ? document.fonts.status : 'unsupported',
    fontFaces: fontFaces,
    fontFaceErrors: fontFaceErrors,
    fontResources: fontResources,
    resources: resources,
    images: Array.from(document.images).map(function (image) {
      return { src: image.currentSrc || image.src, complete: image.complete, width: image.naturalWidth, height: image.naturalHeight };
    }),
    svgImages: svg ? Array.from(svg.querySelectorAll('image')).map(function (image) {
      return { href: image.getAttribute('href') || image.getAttribute('xlink:href'), bounds: rect(image), visible: visible(image) };
    }) : [],
    externalReferences: externalReferences
  };
})()
