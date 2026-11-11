(() => {
  const root = document.documentElement;
  const themeButton = document.querySelector('.theme-toggle');
  themeButton.hidden = false;
  themeButton.addEventListener('click', () => {
    const dark = root.dataset.theme ? root.dataset.theme === 'dark' : matchMedia('(prefers-color-scheme: dark)').matches;
    root.dataset.theme = dark ? 'light' : 'dark';
    try { localStorage.setItem('elintp-theme', root.dataset.theme); } catch { /* Reading still works when storage is disabled. */ }
  });

  document.querySelectorAll('.notes-toggle').forEach(button => {
    const notes = document.getElementById(button.getAttribute('aria-controls'));
    button.hidden = false;
    button.setAttribute('aria-expanded', 'false');
    notes.hidden = true;
    button.addEventListener('click', () => {
      notes.hidden = !notes.hidden;
      button.setAttribute('aria-expanded', String(!notes.hidden));
    });
  });

  // Measure each embedded reader the way page.css prints it: 840px wide, with the reader's own print rules applied.
  addEventListener('beforeprint', () => {
    for (const frame of document.querySelectorAll('.reader-frame iframe')) {
      const doc = frame.contentDocument;
      const printRules = [...doc.styleSheets].flatMap(sheet => [...sheet.cssRules]).filter(rule => rule.media?.mediaText === 'print');
      for (const rule of printRules) rule.media.mediaText = 'all';
      frame.style.width = '840px';
      // The reader's spacing can change with its viewport height, so let the height settle before the final reading.
      for (let pass = 0; pass < 3; pass++) frame.style.height = `${doc.documentElement.scrollHeight}px`;
      frame.parentNode.style.setProperty('--reader-height', doc.documentElement.scrollHeight);
      frame.style.width = frame.style.height = '';
      for (const rule of printRules) rule.media.mediaText = 'print';
    }
  });

  const links = [...document.querySelectorAll('.outline a')];
  if (!links.length) return;
  const sections = [...document.querySelectorAll('main > section')];
  const markCurrent = () => {
    let current = sections[0];
    for (const section of sections) if (section.getBoundingClientRect().top <= 160) current = section;
    for (const link of links) {
      if (link.hash === `#${current.id}`) link.setAttribute('aria-current', 'true');
      else link.removeAttribute('aria-current');
    }
  };
  addEventListener('scroll', markCurrent, { passive: true });
  markCurrent();
})();
