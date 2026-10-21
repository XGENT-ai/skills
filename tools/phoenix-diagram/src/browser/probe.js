// Adapted from Archify 3.1.0 archify/bin/visual-check.mjs::inspect.
// Copyright (c) 2026 tt-a1i (Archify), (c) 2025 Cocoon AI. MIT.
// Phoenix changes: DOM/control/resource coverage, both themes, narrow sample,
// strict finite values, complete semantic paint/depth inspection (initial tree
// folds included) and restored document-flow checks. This measures, not reviews.
(async function () {
  'use strict';
  var runtime = window.PhoenixDiagram;
  var frames = function () {
    return new Promise(function (resolve) {
      requestAnimationFrame(function () { requestAnimationFrame(resolve); });
    });
  };
  var stable = async function () {
    if (runtime) {
      if (runtime.layoutStability && typeof runtime.layoutStability.whenStable === 'function') {
        await runtime.layoutStability.whenStable();
      } else {
        // The two independently scheduled owners can invalidate each other once.
        for (var pass = 0; pass < 2; pass += 1) {
          if (runtime.readerLayout && typeof runtime.readerLayout.whenStable === 'function') await runtime.readerLayout.whenStable();
          if (runtime.viewerChromeLayout && typeof runtime.viewerChromeLayout.whenStable === 'function') await runtime.viewerChromeLayout.whenStable();
        }
      }
    }
    await frames();
  };
  await (document.fonts ? document.fonts.ready : Promise.resolve());
  await stable();
  // An authored initial fold hides a tree branch only in the reading state:
  // measure the whole tree, then return the reader to that fold.
  var treeBranches = runtime && runtime.treeBranches;
  var initialFold = treeBranches && treeBranches.active === true ? treeBranches.collapsedIds() : [];
  if (initialFold.length) { treeBranches.restoreAll(); await stable(); }
  var html = document.documentElement;
  var reader = document.querySelector('.container');
  var diagram = document.querySelector('.diagram-container');
  var svg = diagram && (diagram.querySelector(':scope > svg') || diagram.querySelector(':scope > .diagram-stage > svg'));
  var stage = diagram && (diagram.querySelector(':scope > .diagram-stage') || svg);
  var viewBox = svg && svg.viewBox && svg.viewBox.baseVal;
  var rect = function (element) {
    if (!element) return null;
    var value = element.getBoundingClientRect();
    return { x: value.x, y: value.y, top: value.top, right: value.right, bottom: value.bottom, left: value.left, width: value.width, height: value.height };
  };
  var visible = function (element) {
    if (!element || element.hidden || element.closest('[hidden]')) return false;
    var style = getComputedStyle(element);
    return style.display !== 'none' && style.visibility !== 'hidden' && Number(style.opacity) !== 0 && element.getClientRects().length > 0;
  };
  var intersectionArea = function (a, b) {
    return !a || !b ? 0 : Math.max(0, Math.min(a.right, b.right) - Math.max(a.left, b.left)) * Math.max(0, Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top));
  };
  var contains = function (outer, inner, tolerance) {
    return outer && inner && inner.left >= outer.left - tolerance && inner.right <= outer.right + tolerance && inner.top >= outer.top - tolerance && inner.bottom <= outer.bottom + tolerance;
  };
  var outerHeight = function (element) {
    if (!visible(element)) return 0;
    var style = getComputedStyle(element);
    return element.getBoundingClientRect().height + (parseFloat(style.marginTop) || 0) + (parseFloat(style.marginBottom) || 0);
  };
  // Overflow belongs to each actual viewport ancestor, not the SVG user-space
  // box. Reader can deliberately move its original legend outside that box.
  var overflowClips = function (element, bounds, axes) {
    var failures = [];
    for (var ancestor = element && element.parentElement; ancestor; ancestor = ancestor.parentElement) {
      var style = getComputedStyle(ancestor), box = rect(ancestor);
      if (!box || style.display === 'contents') continue;
      var left = box.left + (parseFloat(style.borderLeftWidth) || 0);
      var right = box.right - (parseFloat(style.borderRightWidth) || 0);
      var top = box.top + (parseFloat(style.borderTopWidth) || 0);
      var bottom = box.bottom - (parseFloat(style.borderBottomWidth) || 0);
      var clips = function (overflow) { return ['hidden', 'clip', 'auto', 'scroll'].includes(overflow); };
      if (axes.x && clips(style.overflowX) && (bounds.left < left - 1 || bounds.right > right + 1)) failures.push({ reason: 'ancestor-overflow-x', tag: ancestor.tagName, bounds: box });
      if (axes.y && clips(style.overflowY) && (bounds.top < top - 1 || bounds.bottom > bottom + 1)) failures.push({ reason: 'ancestor-overflow-y', tag: ancestor.tagName, bounds: box });
    }
    return failures;
  };
  // Reading Depth intentionally hides context/fine at the fitted overview.
  // Inspect every authored object rather than discarding CSS-hidden entries.
  // Camera viewport clipping at Full is legitimate; these observations concern
  // actual computed paint, including inherited/ancestor hiding, not screen fit.
  var semanticVisibility = { ok: false, initialDetailLevel: diagram && diagram.getAttribute('data-detail-level'), inspectionDetailLevel: null,
    initial: { nodes: [], edges: [], text: [], context: [], fine: [] }, inspection: { context: [], fine: [] }, restoration: null, failures: [] };
  var paint = function (element, kind) {
    var bounds = rect(element), reasons = [], style = element && getComputedStyle(element);
    if (!element) reasons.push('missing');
    for (var ancestor = element; ancestor; ancestor = ancestor.parentElement) {
      var inherited = getComputedStyle(ancestor);
      if (ancestor.hidden || inherited.display === 'none' || ['hidden', 'collapse'].includes(inherited.visibility) || !(Number(inherited.opacity) > 0)) {
        reasons.push('hidden:' + ancestor.tagName.toLowerCase()); break;
      }
    }
    if (!bounds || ![bounds.x, bounds.y, bounds.width, bounds.height].every(Number.isFinite) || !element.getClientRects().length ||
        (kind === 'edge' ? !(bounds.width > 0 || bounds.height > 0) : !(bounds.width > 0 && bounds.height > 0))) reasons.push('empty-bounds');
    var colorVisible = function (color) { return Boolean(color && color !== 'none' && color !== 'transparent' && !/^rgba\([^)]*,\s*0(?:\.0+)?\s*\)$/.test(color) && !/\/\s*0(?:\.0+)?\s*\)$/.test(color)); };
    var fill = Boolean(style && colorVisible(style.fill) && Number(style.fillOpacity) > 0);
    var stroke = Boolean(style && colorVisible(style.stroke) && Number(style.strokeOpacity) > 0 && parseFloat(style.strokeWidth) > 0);
    if (kind === 'edge' ? !stroke : !fill && !stroke) reasons.push('no-paint');
    var length = null;
    if (kind === 'edge' && element) {
      try { length = element.getTotalLength(); } catch (_) {}
      if (!element.getAttribute('d') || !(Number.isFinite(length) && length > 0)) reasons.push('empty-path');
    }
    return { visible: reasons.length === 0, bounds: bounds, reasons: reasons, length: length };
  };
  var observeText = function (element) {
    return Object.assign({ text: (element.textContent || '').trim(), sourcePath: element.getAttribute('data-source-path') }, paint(element, 'text'));
  };
  var authoredText = svg ? Array.from(svg.querySelectorAll('text[data-source-path]')).filter(function (element) { return (element.textContent || '').trim(); }) : [];
  var depth = function (element) { var owner = element.closest('[data-detail]'); return owner && owner.getAttribute('data-detail'); };
  if (svg) {
    semanticVisibility.initial.nodes = Array.from(svg.querySelectorAll('[data-node-id]')).map(function (node) {
      var bodies = Array.from(node.querySelectorAll(':scope > [data-graph-role="node-body"]')).map(function (body) { return paint(body, 'body'); });
      var labels = Array.from(node.querySelectorAll('text[data-node-label]')).map(observeText);
      var entry = { id: node.getAttribute('data-node-id'), bodies: bodies, labels: labels };
      entry.visible = bodies.length > 0 && labels.length > 0 && bodies.every(function (body) { return body.visible; }) && labels.every(function (label) { return label.visible; });
      return entry;
    });
    semanticVisibility.initial.edges = Array.from(svg.querySelectorAll('path[data-graph-role="edge"][data-edge-from][data-edge-to][data-edge-key]')).map(function (edge) {
      return Object.assign({ key: edge.getAttribute('data-edge-key'), id: edge.getAttribute('data-edge-id'), from: edge.getAttribute('data-edge-from'), to: edge.getAttribute('data-edge-to') }, paint(edge, 'edge'));
    });
    semanticVisibility.initial.text = authoredText.filter(function (element) { return !['context', 'fine'].includes(depth(element)); }).map(observeText);
    semanticVisibility.initial.context = authoredText.filter(function (element) { return depth(element) === 'context'; }).map(observeText);
    semanticVisibility.initial.fine = authoredText.filter(function (element) { return depth(element) === 'fine'; }).map(observeText);
  }
  var graph = runtime && runtime.metadata && runtime.metadata.graph;
  if (!graph || !Array.isArray(graph.nodes) || !Array.isArray(graph.relations)) {
    semanticVisibility.failures.push({ kind: 'identity', reason: 'authored-graph-unavailable' });
  } else {
    var expectedNodes = graph.nodes.map(function (node) { return node.id; });
    var expectedEdges = graph.relations.map(function (relation, index) { return { key: String(index), id: typeof relation.id === 'string' ? relation.id : null, from: relation.from, to: relation.to }; });
    semanticVisibility.expected = { nodeIds: expectedNodes, relations: expectedEdges };
    if (expectedNodes.some(function (id) { return typeof id !== 'string' || !id; }) || new Set(expectedNodes).size !== expectedNodes.length || semanticVisibility.initial.nodes.length !== expectedNodes.length) {
      semanticVisibility.failures.push({ kind: 'identity', reason: 'node-identity-count', expected: expectedNodes, actual: semanticVisibility.initial.nodes.map(function (node) { return node.id; }) });
    }
    expectedNodes.forEach(function (id) {
      if (semanticVisibility.initial.nodes.filter(function (node) { return node.id === id; }).length !== 1) semanticVisibility.failures.push({ kind: 'identity', reason: 'node-identity-missing-or-duplicate', id: id });
    });
    if (semanticVisibility.initial.edges.length !== expectedEdges.length) semanticVisibility.failures.push({ kind: 'identity', reason: 'relation-identity-count', expected: expectedEdges.length, actual: semanticVisibility.initial.edges.length });
    expectedEdges.forEach(function (expected) {
      var matches = semanticVisibility.initial.edges.filter(function (edge) { return edge.key === expected.key; });
      if (matches.length !== 1 || matches[0].from !== expected.from || matches[0].to !== expected.to || matches[0].id !== expected.id) {
        semanticVisibility.failures.push({ kind: 'identity', reason: 'relation-identity-missing-duplicate-or-mismatched', expected: expected, actual: matches });
      }
    });
  }
  var view = runtime && runtime.view;
  var beforeInspection = { scrollX: window.scrollX, scrollY: window.scrollY, bounds: rect(svg), camera: view && typeof view.state === 'function' ? view.state() : null };
  var fittedBounds = null;
  if (!view || typeof view.zoomIn !== 'function' || typeof view.reset !== 'function' || typeof view.state !== 'function') {
    semanticVisibility.failures.push({ kind: 'inspection', reason: 'public-camera-unavailable' });
  } else {
    try {
      // Reset establishes the source camera's 100% fitted baseline even if its
      // initial Reading Depth was Map. Three public 25% steps must reach Full.
      view.reset({ automatic: true });
      await stable();
      fittedBounds = rect(svg);
      view.zoomIn(); view.zoomIn(); view.zoomIn();
      await stable();
      semanticVisibility.inspectionDetailLevel = diagram.getAttribute('data-detail-level');
      semanticVisibility.inspection.context = authoredText.filter(function (element) { return depth(element) === 'context'; }).map(observeText);
      semanticVisibility.inspection.fine = authoredText.filter(function (element) { return depth(element) === 'fine'; }).map(observeText);
      if (semanticVisibility.inspectionDetailLevel !== 'full') semanticVisibility.failures.push({ kind: 'inspection', reason: 'full-depth-unavailable' });
    } finally {
      view.reset({ automatic: true });
      await stable();
      window.scrollTo(beforeInspection.scrollX, beforeInspection.scrollY);
      await frames();
      var restored = view.state();
      var restoredBounds = rect(svg);
      semanticVisibility.restoration = { camera: restored, previousCamera: beforeInspection.camera, detailLevel: diagram.getAttribute('data-detail-level'),
        bounds: restoredBounds, previousBounds: beforeInspection.bounds, fittedBounds: fittedBounds,
        ok: restored.mode === 'overview' && restored.scale === 1 && restored.x === 0 && restored.y === 0 && fittedBounds && restoredBounds &&
          ['x', 'y', 'width', 'height'].every(function (key) { return Math.abs(restoredBounds[key] - fittedBounds[key]) <= 1; }) };
      if (!semanticVisibility.restoration.ok) semanticVisibility.failures.push({ kind: 'restoration', reason: 'fitted-overview-not-restored' });
    }
  }
  ['nodes', 'edges', 'text'].forEach(function (kind) {
    semanticVisibility.initial[kind].forEach(function (entry) { if (!entry.visible) semanticVisibility.failures.push({ kind: kind, entry: entry }); });
  });
  ['context', 'fine'].forEach(function (kind) {
    semanticVisibility.inspection[kind].forEach(function (entry) { if (!entry.visible) semanticVisibility.failures.push({ kind: kind, entry: entry }); });
  });
  semanticVisibility.ok = Boolean(svg && semanticVisibility.initial.nodes.length && semanticVisibility.restoration && semanticVisibility.restoration.ok && !semanticVisibility.failures.length);
  var svgRect = rect(svg);
  var diagramRect = rect(diagram);
  var stageRect = runtime && runtime.viewerChromeLayout && typeof runtime.viewerChromeLayout.stageRect === 'function'
    ? runtime.viewerChromeLayout.stageRect() : rect(stage);
  var dock = diagram && diagram.querySelector('.diagram-nav');
  var dockRect = runtime && runtime.viewerChromeLayout && typeof runtime.viewerChromeLayout.dockRect === 'function'
    ? runtime.viewerChromeLayout.dockRect() : rect(dock);
  var chromeReceipt = runtime && runtime.viewerChromeLayout && typeof runtime.viewerChromeLayout.receipt === 'function'
    ? runtime.viewerChromeLayout.receipt() : null;
  var legend = svg && svg.querySelector('[data-legend]');
  var legendRect = rect(legend);
  var diagramWidth = svgRect ? svgRect.width : 0;
  var scale = viewBox && viewBox.width > 0 ? Math.min(1, diagramWidth / viewBox.width) : 0;
  var text = [];
  var nodeClips = [];
  if (svg && scale > 0) {
    Array.from(svg.querySelectorAll('text[data-node-label], text[data-boundary-label], text[data-detail="context"], g[data-detail="context"] text')).forEach(function (element) {
      if (element.getAttribute('data-detail') === 'fine' || element.closest('[data-detail="fine"]') || !visible(element)) return;
      var edge = element.closest('[data-edge-from][data-edge-to]');
      var node = element.closest('[data-node-id]');
      var context = element.getAttribute('data-detail') === 'context' || Boolean(element.closest('g[data-detail="context"]'));
      var detail = element.hasAttribute('data-node-label') ? 'primary' : element.hasAttribute('data-boundary-label') ? 'boundary' : context && edge ? 'edge' : 'context';
      if (detail === 'context' && !node) return;
      var font = parseFloat(element.getAttribute('font-size') || getComputedStyle(element).fontSize);
      var bounds = rect(element);
      text.push({ text: (element.textContent || '').trim(), detail: detail, sourceFontPx: font, projectedFontPx: font * scale, bounds: bounds,
        owner: edge ? { kind: 'edge', id: edge.getAttribute('data-edge-id'), from: edge.getAttribute('data-edge-from'), to: edge.getAttribute('data-edge-to') }
          : node ? { kind: 'node', id: node.getAttribute('data-node-id') } : { kind: 'boundary', id: null } });
      if (node) {
        // A waterfall row frames its name column and span bar.
        var box = node.querySelector(':scope > rect.wf-row') || node.querySelector(':scope > [data-graph-role="node-body"]') || node.querySelector(':scope > rect:not(.c-mask)') || node.querySelector(':scope > rect');
        if (box && !contains(rect(box), bounds, 1)) nodeClips.push({ id: node.getAttribute('data-node-id'), text: (element.textContent || '').trim(), box: rect(box), textBounds: bounds });
      }
    });
  }
  var minimum = text.reduce(function (best, entry) { return !best || entry.projectedFontPx < best.projectedFontPx ? entry : best; }, null);
  var canvasTextChecks = svg ? Array.from(svg.querySelectorAll('text')).filter(function (element) {
    return element.closest('[data-legend]') || visible(element) && !element.closest('[data-detail="fine"]');
  }).map(function (element) {
    var bounds = rect(element), legendOwner = element.closest('[data-legend]');
    var migrated = legendOwner === legend && legendOwner && legendOwner.hasAttribute('data-reader-legend-corner');
    var failures = [];
    if (legendOwner && !paint(element, 'text').visible) failures.push({ reason: 'legend-text-not-painted' });
    if (!contains(migrated ? diagramRect : svgRect, bounds, 1)) failures.push({ reason: migrated ? 'outside-reader-panel' : 'outside-svg' });
    if (migrated) {
      failures = failures.concat(overflowClips(element, bounds, { x: true, y: true }));
      if (!contains({ left: 0, top: 0, right: window.innerWidth, bottom: window.innerHeight }, bounds, 1)) failures.push({ reason: 'outside-viewport' });
    }
    return { text: (element.textContent || '').trim(), bounds: bounds, container: migrated ? 'reader-panel' : 'svg', failures: failures };
  }) : [];
  var canvasTextClips = canvasTextChecks.filter(function (entry) { return entry.failures.length > 0; });
  var cards = Array.from(document.querySelectorAll('.cards > .card, .cards > [data-card]')).filter(visible).map(function (element) {
    return { id: element.id || null, bounds: rect(element), scrollWidth: element.scrollWidth, clientWidth: element.clientWidth, scrollHeight: element.scrollHeight, clientHeight: element.clientHeight,
      overlapWithSvg: intersectionArea(rect(element), svgRect), clippedText: element.scrollWidth > element.clientWidth + 1 || element.scrollHeight > element.clientHeight + 1 };
  });
  var selectors = ['#btn-theme', '#btn-preset', '#btn-export', '#btn-node-finder', '[data-view="out"]', '[data-view="reset"]', '[data-view="in"]'];
  var controls = [];
  var savedScroll = { x: window.scrollX, y: window.scrollY };
  for (var selector of selectors) {
    var control = document.querySelector(selector);
    if (!control || !visible(control)) { controls.push({ selector: selector, present: Boolean(control), visible: false, hit: false }); continue; }
    control.scrollIntoView({ block: 'center', inline: 'nearest', behavior: 'instant' });
    await frames();
    var bounds = rect(control);
    var x = bounds.left + bounds.width / 2;
    var y = bounds.top + bounds.height / 2;
    var hit = document.elementFromPoint(x, y);
    controls.push({ selector: selector, present: true, visible: true, disabled: Boolean(control.disabled), bounds: bounds, hit: Boolean(hit && (hit === control || control.contains(hit))), focusable: control.tabIndex >= 0, accessibleName: control.getAttribute('aria-label') || control.getAttribute('title') || (control.textContent || '').trim() });
  }
  window.scrollTo(savedScroll.x, savedScroll.y);
  await frames();
  // The narrow rail is display:contents and has no rectangle or mode attribute.
  // Only a measured, reachable normal-flow tail may leave the diagram's first
  // screen budget. A clipped tail must not make the page appear to fit.
  var rail = reader && reader.querySelector(':scope > .reader-rail');
  var noteElements = rail ? Array.from(rail.querySelectorAll(':scope > .cards, :scope > .node-outline')).filter(function (element) {
    var bounds = rect(element);
    return visible(element) && bounds && bounds.width > 0 && bounds.height > 0;
  }) : [];
  var notes = noteElements.map(function (element) {
    var bounds = rect(element), normal = true;
    for (var ancestor = element; ancestor && ancestor !== reader; ancestor = ancestor.parentElement) {
      if (!['static', 'relative'].includes(getComputedStyle(ancestor).position)) normal = false;
    }
    return { kind: element.classList.contains('cards') ? 'cards' : 'index', bounds: bounds, normalFlow: normal };
  });
  var notesBelowFold = Boolean(diagramRect && notes.length && notes.every(function (note) { return note.normalFlow && note.bounds.top >= diagramRect.bottom - 1; }));
  var flowFailures = svgRect && diagramRect ? overflowClips(svg, svgRect, { x: false, y: true }) : [{ reason: 'missing-diagram-bounds' }];
  if (svgRect && diagramRect && (svgRect.top < diagramRect.top - 1 || svgRect.bottom > diagramRect.bottom + 1)) flowFailures.push({ reason: 'diagram-vertical-clip' });
  var notesBottom = notesBelowFold ? Math.max.apply(null, notes.map(function (note) { return note.bounds.bottom; })) : null;
  if (notesBelowFold) {
    noteElements.forEach(function (element, index) { flowFailures = flowFailures.concat(overflowClips(element, notes[index].bounds, { x: true, y: true })); });
    if (notesBottom + window.scrollY > html.scrollHeight + 1) flowFailures.push({ reason: 'notes-outside-document' });
    if (notesBottom > window.innerHeight + 1 && [html, document.body].some(function (element) { return ['hidden', 'clip'].includes(getComputedStyle(element).overflowY); })) flowFailures.push({ reason: 'notes-document-scroll-disabled' });
  }
  var excludedNotesHeight = notesBelowFold && !flowFailures.length ? notesBottom - diagramRect.bottom : 0;
  var readerFlow = { unclipped: !flowFailures.length, notesBelowDiagram: notesBelowFold, notes: notes, excludedHeight: excludedNotesHeight, failures: flowFailures };
  var documentScrollUnclipped = Boolean(svgRect && diagramRect && contains(diagramRect, svgRect, 1)
    && diagram.scrollHeight <= diagram.clientHeight + 1 && diagram.scrollWidth <= diagram.clientWidth + 1
    && [html, document.body].every(function (element) { return !['hidden', 'clip'].includes(getComputedStyle(element).overflowY); }));
  if (initialFold.length) {
    initialFold.forEach(function (id) { treeBranches.collapse(id); });
    await stable();
    var refolded = treeBranches.collapsedIds();
    semanticVisibility.treeFold = { unfoldedForInspection: initialFold, restored: refolded };
    if (refolded.length !== initialFold.length || initialFold.some(function (id) { return refolded.indexOf(id) < 0; })) {
      semanticVisibility.failures.push({ kind: 'restoration', reason: 'tree-fold-not-restored' });
      semanticVisibility.ok = false;
    }
  }
  var media = Array.from(document.querySelectorAll('audio,video')).map(function (element) { return { tag: element.tagName, paused: element.paused, ended: element.ended, error: element.error ? { code: element.error.code, message: element.error.message } : null }; });
  return {
    documentUrl: window.location.href,
    innerWidth: window.innerWidth, innerHeight: window.innerHeight,
    scrollWidth: Math.ceil(Math.max(html.scrollWidth, document.body.scrollWidth)),
    scrollHeight: Math.max(window.innerHeight, Math.ceil(html.scrollHeight - excludedNotesHeight)),
    fullScrollHeight: Math.ceil(html.scrollHeight),
    resolvedTheme: html.getAttribute('data-theme') || '',
    readerLayout: html.getAttribute('data-reader-layout'), readerOverflow: html.getAttribute('data-reader-overflow'),
    readerFit: svg ? svg.getAttribute('data-reader-fit') : null, diagramType: svg ? svg.getAttribute('data-diagram-type') : null,
    documentScrollUnclipped: documentScrollUnclipped,
    readerFlow: readerFlow,
    readerRect: rect(reader), panelRect: diagramRect, svgRect: svgRect, stageRect: stageRect,
    viewBox: viewBox ? { x: viewBox.x, y: viewBox.y, width: viewBox.width, height: viewBox.height } : null,
    svgCount: diagram ? diagram.querySelectorAll(':scope > svg, :scope > .diagram-stage > svg').length : 0,
    nodeCount: svg ? svg.querySelectorAll('[data-node-id]').length : 0,
    edgeCount: svg ? svg.querySelectorAll('[data-graph-role="edge"]').length : 0,
    semanticVisibility: semanticVisibility,
    minimumProjectedNodeTextPx: minimum ? minimum.projectedFontPx : null,
    minimumProjectedNodeText: minimum,
    text: text, nodeTextClips: nodeClips, canvasTextClips: canvasTextClips, canvasTextChecks: canvasTextChecks, cards: cards, controls: controls,
    fontsStatus: document.fonts ? document.fonts.status : 'unsupported',
    images: Array.from(document.images).map(function (image) { return { src: image.currentSrc || image.src, complete: image.complete, width: image.naturalWidth, height: image.naturalHeight }; }),
    resources: performance.getEntriesByType('resource').map(function (entry) { return { name: entry.name, initiatorType: entry.initiatorType }; }),
    media: media,
    capabilities: { canvas: Boolean(document.createElement('canvas').getContext('2d')), mediaRecorder: typeof MediaRecorder !== 'undefined', webm: typeof MediaRecorder !== 'undefined' && MediaRecorder.isTypeSupported('video/webm'), clipboard: Boolean(navigator.clipboard && navigator.clipboard.write), clipboardItem: typeof ClipboardItem !== 'undefined' },
    motion: html.getAttribute('data-motion'),
    hasLegend: Boolean(legendRect && legendRect.width && legendRect.height), hasNavigationDock: Boolean(dockRect && dockRect.width && dockRect.height),
    legendDockIntersectionArea: intersectionArea(stageRect, dockRect) > 0 ? intersectionArea(legendRect, dockRect) : 0,
    dockStageIntersectionArea: intersectionArea(stageRect, dockRect), dockStageGap: stageRect && dockRect ? dockRect.top - stageRect.bottom : null,
    viewerChromeRequiredGap: chromeReceipt ? chromeReceipt.gap : null,
    pageComposition: { headerPx: Math.round(outerHeight(reader && reader.querySelector('.header'))), svgPx: svgRect ? Math.round(svgRect.height) : 0, cardsPx: notesBelowFold ? 0 : Math.round(outerHeight(reader && reader.querySelector('.cards'))) }
  };
})()
