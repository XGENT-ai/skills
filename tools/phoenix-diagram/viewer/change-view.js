/*
Phoenix Diagram reader fragment: change views.
Copyright (c) 2025 Cocoon AI.
Full license: LICENSE.txt.
*/
    /* ============================================================
       Change Views — three views over one input's change marks. The
       renderer emits the changes view by default; Before and After
       hide the elements that do not exist there. Views are reading
       state only: never stored, never written to share links, and
       export and print always render the selected view. Explorations
       (Route, reachability) return the reader to the changes view so
       marks stay visible alongside their results.
       ============================================================ */
    PhoenixDiagram.changeView = (function () {
      var svg = document.querySelector('.diagram-container > svg[data-change-ui]');
      var group = document.getElementById('change-view-switch');
      if (!svg || !group) return { active: false, reveal: function () { return false; } };
      var status = document.getElementById('change-view-status');
      group.hidden = false;
      var buttons = {};
      Array.prototype.forEach.call(group.querySelectorAll('[data-change-view-select]'), function (button) {
        buttons[button.getAttribute('data-change-view-select')] = button;
      });
      var current = svg.getAttribute('data-change-view') || 'changes';

      function set(view) {
        if (!buttons[view]) return false;
        current = view;
        svg.setAttribute('data-change-view', view);
        Object.keys(buttons).forEach(function (name) {
          buttons[name].setAttribute('aria-pressed', name === view ? 'true' : 'false');
        });
        return true;
      }

      function returned() {
        if (status) status.textContent = group.getAttribute('data-change-view-returned') || '';
      }

      // A focused element hidden in the current view (an addition in Before,
      // a removal in After) pulls the reader back to the changes view.
      function reveal(id) {
        var node = svg.querySelector('[data-node-id="' + id + '"]');
        if (!node || !node.getAttribute('data-change')) return false;
        var view = svg.getAttribute('data-change-view') || 'changes';
        var mark = node.getAttribute('data-change');
        var hidden = (view === 'before' && mark === 'added') ||
          (view === 'after' && mark === 'removed');
        if (!hidden) return false;
        set('changes');
        returned();
        return true;
      }

      Array.prototype.forEach.call(group.querySelectorAll('[data-change-view-select]'), function (button) {
        button.addEventListener('click', function () {
          set(button.getAttribute('data-change-view-select'));
        });
      });

      new MutationObserver(function () {
        if (current !== 'changes' &&
            (svg.hasAttribute('data-route-active') || svg.hasAttribute('data-reach-active'))) {
          set('changes');
          returned();
        }
      }).observe(svg, { attributes: true, attributeFilter: ['data-route-active', 'data-reach-active'] });

      return { active: true, view: function () { return current; }, set: set, reveal: reveal };
    })();
