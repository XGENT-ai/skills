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

  const links = [...document.querySelectorAll('.outline a')];
  if (!links.length) return;
  const sections = [...document.querySelectorAll('main > section')];
  const markCurrent = () => {
    let current = sections[0];
    for (const section of sections) if (section.getBoundingClientRect().top <= 160) current = section;
    if (innerHeight + scrollY >= document.documentElement.scrollHeight - 2) current = sections.at(-1);
    for (const link of links) {
      if (link.hash === `#${current.id}`) link.setAttribute('aria-current', 'true');
      else link.removeAttribute('aria-current');
    }
  };
  const observer = new IntersectionObserver(markCurrent, { rootMargin: '-5% 0px -65% 0px', threshold: [0, 1] });
  sections.forEach(section => observer.observe(section));
  addEventListener('hashchange', markCurrent);
  markCurrent();
})();
