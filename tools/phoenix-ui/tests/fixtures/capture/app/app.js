const $ = (id) => document.getElementById(id);
// The demo analytics script is missing on purpose: one console error at load.
console.error('统计脚本加载失败：analytics.js 不存在');

$('login-form').addEventListener('submit', (event) => {
  event.preventDefault();
  if (!$('username').value.trim() || !$('password').value) {
    $('login-error').hidden = false;
    return;
  }
  $('login').hidden = true;
  $('tasks').hidden = false;
});

$('filter-button').addEventListener('click', () => {
  const panel = $('filter-panel');
  panel.hidden = !panel.hidden;
  $('filter-button').setAttribute('aria-expanded', String(!panel.hidden));
});

$('new-button').addEventListener('click', () => {
  $('new-form').hidden = false;
  $('title').focus();
});

$('new-form').addEventListener('submit', async (event) => {
  event.preventDefault();
  const title = $('title').value.trim();
  $('title-error').hidden = Boolean(title);
  if (!title) return;
  $('save-status').textContent = '正在保存…';
  try {
    const response = await fetch('/api/tasks', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ title }),
    });
    if (!response.ok) throw new Error(String(response.status));
    $('save-status').textContent = '已保存「' + title + '」';
  } catch {
    $('save-status').textContent = '保存失败，请重试';
  }
});
