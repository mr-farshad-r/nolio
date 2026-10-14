'use strict';
const $ = (s, el) => (el || document).querySelector(s);
const $$ = (s, el) => Array.from((el || document).querySelectorAll(s));
const J = { 'Content-Type': 'application/json' };

async function req(method, url, body) {
  const r = await fetch(url, { method, headers: body ? J : {}, body: body ? JSON.stringify(body) : undefined });
  if (!r.ok) throw new Error(method + ' ' + url + ' -> ' + r.status);
  const t = await r.text();
  return t ? JSON.parse(t) : null;
}

const api = {
  pages: () => req('GET', '/api/pages'),
  getPage: id => req('GET', '/api/pages/' + id),
  createPage: b => req('POST', '/api/pages', b),
  updatePage: (id, b) => req('PATCH', '/api/pages/' + id, b),
  deletePage: id => req('DELETE', '/api/pages/' + id),
  blocks: pid => req('GET', '/api/blocks/' + pid),
  createBlock: (pid, b) => req('POST', '/api/blocks/' + pid, b),
  updateBlock: (id, b) => req('PATCH', '/api/blocks/block/' + id, b),
  deleteBlock: id => req('DELETE', '/api/blocks/block/' + id),
  moveBlock: (id, b) => fetch('/api/blocks/block/' + id + '/move', { method: 'POST', headers: J, body: JSON.stringify(b) }),
  databases: () => req('GET', '/api/databases'),
  createDb: b => req('POST', '/api/databases', b),
  getDb: id => req('GET', '/api/databases/' + id),
  rows: id => req('GET', '/api/databases/' + id + '/rows'),
  createRow: (id, b) => req('POST', '/api/databases/' + id + '/rows', b || {}),
  addProp: (id, b) => req('POST', '/api/databases/' + id + '/props', b),
  setProp: (id, pid, propId, v) => req('PUT', '/api/databases/' + id + '/rows/' + pid + '/props/' + propId, v),
};

const state = { pages: [], current: null, currentDb: null, collapsed: {}, search: '' };

// ---------- sidebar ----------
async function refreshSidebar() {
  state.pages = await api.pages();
  renderTree();
  renderFavs();
  renderDbs();
}

function pageChildren(parentId) {
  return state.pages.filter(p => (p.parent_id || null) === parentId && matchesSearch(p));
}
function matchesSearch(p) {
  if (!state.search) return true;
  if ((p.title || '').toLowerCase().includes(state.search)) return true;
  // keep ancestors of matches
  return pageChildren(p.id).length > 0;
}

function renderTree() {
  const root = $('#page-tree');
  root.innerHTML = '';
  renderLevel(root, null, 0);
}
function renderLevel(container, parentId, depth) {
  for (const p of pageChildren(parentId)) {
    container.appendChild(pageItem(p, depth));
    if (!state.collapsed[p.id] && pageChildren(p.id).length) {
      renderLevel(container, p.id, depth + 1);
    }
  }
}
function pageItem(p, depth) {
  const el = document.createElement('div');
  el.className = 'page-item' + (p.id === state.current ? ' active' : '');
  el.style.paddingLeft = 6 + depth * 16 + 'px';
  const kids = state.pages.filter(q => q.parent_id === p.id);
  const caret = document.createElement('span');
  caret.className = 'caret';
  caret.textContent = kids.length ? (state.collapsed[p.id] ? '▸' : '▾') : '';
  caret.onclick = e => { e.stopPropagation(); state.collapsed[p.id] = !state.collapsed[p.id]; renderTree(); };
  el.appendChild(caret);
  const icon = document.createElement('span'); icon.className = 'picon'; icon.textContent = p.icon || '📄'; el.appendChild(icon);
  const name = document.createElement('span'); name.className = 'pname'; name.textContent = p.title || 'Untitled'; el.appendChild(name);
  const act = document.createElement('span'); act.className = 'actions';
  const plus = document.createElement('span'); plus.textContent = '+';
  plus.onclick = async e => { e.stopPropagation(); const { id } = await api.createPage({ parent_id: p.id }); state.collapsed[p.id] = false; await refreshSidebar(); openPage(id); };
  act.appendChild(plus);
  el.appendChild(act);
  el.onclick = () => openPage(p.id);
  return el;
}

function renderFavs() {
  const favs = state.pages.filter(p => p.favorite);
  $('#favorites').hidden = !favs.length;
  const c = $('#fav-tree'); c.innerHTML = '';
  favs.forEach(p => c.appendChild(pageItem(p, 0)));
}

async function renderDbs() {
  const dbs = await api.databases();
  const el = $('#dblist');
  if (!el) return;
  el.innerHTML = '';
  for (const d of dbs) {
    const item = document.createElement('div');
    item.className = 'page-item' + (d.id === state.currentDb ? ' active' : '');
    item.innerHTML = '<span class="picon">🗃</span><span class="pname"></span>';
    item.querySelector('.pname').textContent = d.name;
    item.onclick = () => openDb(d.id);
    el.appendChild(item);
  }
  const add = document.createElement('div');
  add.className = 'page-item';
  add.innerHTML = '<span class="picon">＋</span><span class="pname">New database</span>';
  add.onclick = async () => { const { id } = await api.createDb({}); await refreshSidebar(); openDb(id); };
  el.appendChild(add);
}

$('#search').addEventListener('input', e => { state.search = e.target.value.toLowerCase(); renderTree(); });
$('#new-page').onclick = async () => {
  const { id } = await api.createPage({});
  await refreshSidebar();
  openPage(id);
};

// ---------- page editor ----------
const debouncers = {};
function debounce(key, fn, ms) {
  clearTimeout(debouncers[key]);
  debouncers[key] = setTimeout(fn, ms || 400);
}

async function openPage(id) {
  state.current = id;
  state.currentDb = null;
  const [meta, blocks] = await Promise.all([api.getPage(id), api.blocks(id)]);
  $('#db-view').hidden = true;
  $('#page-view').hidden = false;
  $('#title').value = meta.title || '';
  document.title = (meta.title || 'Untitled') + ' — Nolio';
  renderBlocks(blocks);
  renderTree();
}

let titleTimer;
$('#title').addEventListener('input', () => {
  debounce('title', async () => {
    if (state.current) { await api.updatePage(state.current, { title: $('#title').value }); refreshSidebar(); }
  });
});

const SLASH = [
  { name: 'Text', type: 'paragraph', ic: 'Aa', desc: 'Plain text' },
  { name: 'Heading 1', type: 'heading_1', ic: 'H1', desc: 'Big section heading' },
  { name: 'Heading 2', type: 'heading_2', ic: 'H2', desc: 'Medium section heading' },
  { name: 'Heading 3', type: 'heading_3', ic: 'H3', desc: 'Small section heading' },
  { name: 'To-do list', type: 'to_do', ic: '☑', desc: 'Track tasks with a checkbox' },
  { name: 'Bulleted list', type: 'bulleted_list', ic: '•', desc: 'Simple bulleted list' },
  { name: 'Numbered list', type: 'numbered_list', ic: '1.', desc: 'Numbered list' },
  { name: 'Toggle list', type: 'toggle', ic: '▸', desc: 'Collapsible content' },
  { name: 'Quote', type: 'quote', ic: '❝', desc: 'Capture a quote' },
  { name: 'Divider', type: 'divider', ic: '—', desc: 'Visual divider' },
  { name: 'Code', type: 'code', ic: '</>', desc: 'Code block' },
  { name: 'Callout', type: 'callout', ic: '💡', desc: 'Highlighted note' },
];

function buildTree(blocks) {
  const childrenOf = {}, roots = [], byId = {};
  blocks.forEach(b => byId[b.id] = b);
  blocks.forEach(b => {
    if (b.parent_id && byId[b.parent_id]) (childrenOf[b.parent_id] = childrenOf[b.parent_id] || []).push(b);
    else roots.push(b);
  });
  const sort = cs => cs.sort((a, b) => a.ord - b.ord);
  Object.values(childrenOf).forEach(sort);
  sort(roots);
  return { roots, childrenOf };
}

function renderBlocks(blocks) {
  const c = $('#blocks');
  c.innerHTML = '';
  const { roots, childrenOf } = buildTree(blocks);
  roots.forEach((b, i) => c.appendChild(blockWrap(b, roots, childrenOf, i)));
  if (!roots.length) c.appendChild(freshParagraph(null, roots));
}

function freshParagraph(parentId, siblings) {
  const wrap = document.createElement('div');
  wrap.className = 'blockwrap';
  const row = document.createElement('div');
  row.className = 'blockrow'; row.dataset.type = 'paragraph';
  const plus = mk('plus', '+');
  plus.onclick = () => showSlashAt(null, parentId);
  row.appendChild(plus);
  row.appendChild(mk('grip', '⠿'));
  const text = document.createElement('div');
  text.className = 'text'; text.contentEditable = 'true'; text.dataset.ph = "Type '/' for commands";
  wireText(text, null, row, parentId);
  row.appendChild(text);
  wrap.appendChild(row);
  return wrap;
}

function mk(cls, t) { const s = document.createElement('span'); s.className = cls; s.textContent = t; return s; }

function blockWrap(b, siblings, childrenOf, idx) {
  const wrap = document.createElement('div');
  wrap.className = 'blockwrap';
  wrap.dataset.id = b.id;
  const row = document.createElement('div');
  row.className = 'blockrow';
  row.dataset.type = b.type;
  row.dataset.id = b.id;
  row.draggable = true;

  row.addEventListener('dragstart', e => { e.dataTransfer.setData('text/plain', b.id); });
  row.addEventListener('dragover', e => { e.preventDefault(); row.classList.add('drag-over'); });
  row.addEventListener('dragleave', () => row.classList.remove('drag-over'));
  row.addEventListener('drop', async e => {
    e.preventDefault(); row.classList.remove('drag-over');
    const dragId = e.dataTransfer.getData('text/plain');
    if (!dragId || dragId === b.id) return;
    const prevOrd = idx > 0 ? siblings[idx - 1].ord : b.ord - 1;
    const newOrd = (prevOrd + b.ord) / 2;
    const r = await api.moveBlock(dragId, { ord: newOrd, parent_id: b.parent_id });
    if (r.ok) reopen(); else alert('Cannot move there.');
  });

  if (b.type === 'divider') { wrap.appendChild(row); return wrap; }

  const plus = mk('plus', '+');
  plus.onclick = () => showSlashAt(b, b.parent_id);
  row.appendChild(plus);
  row.appendChild(mk('grip', '⠿'));

  const kids = childrenOf[b.id] || [];
  if (b.type === 'toggle' || kids.length) {
    const tw = mk('caret-arrow', kids.length || b.type === 'toggle' ? '▾' : '');
    tw.onclick = () => { wrap.classList.toggle('collapsed'); tw.textContent = wrap.classList.contains('collapsed') ? '▸' : '▾'; };
    row.appendChild(tw);
  }
  if (b.type === 'to_do') {
    const cb = document.createElement('input');
    cb.type = 'checkbox'; cb.checked = !!b.content.checked;
    cb.onchange = () => saveBlock(b.id, { checked: cb.checked });
    row.appendChild(cb);
  }
  if (b.type === 'bulleted_list') row.appendChild(mk('bullet', '•'));
  if (b.type === 'numbered_list') row.appendChild(mk('bullet', (idx + 1) + '.'));

  const text = document.createElement('div');
  text.className = 'text' + (b.type === 'to_do' && b.content.checked ? ' done' : '');
  text.contentEditable = 'true';
  text.dataset.ph = "Type '/' for commands";
  text.innerHTML = b.content.html || escapeHtml(b.content.text || '');
  wireText(text, b, row, b.parent_id);
  row.appendChild(text);
  wrap.appendChild(row);

  if (kids.length) {
    const kc = document.createElement('div');
    kc.className = 'children';
    kids.forEach((k, i) => kc.appendChild(blockWrap(k, kids, childrenOf, i)));
    wrap.appendChild(kc);
  }
  return wrap;
}

function escapeHtml(s) {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

async function saveBlock(id, patch) {
  const el = document.querySelector('.blockrow[data-id="' + id + '"] .text');
  const cur = el ? { text: el.textContent, html: el.innerHTML } : {};
  await api.updateBlock(id, { content: Object.assign(cur, patch) });
}

async function reopen() {
  if (state.currentDb) { renderDbView(); return; }
  const blocks = await api.blocks(state.current);
  renderBlocks(blocks);
}

function focusBlock(id) {
  const el = document.querySelector('.blockrow[data-id="' + id + '"] .text');
  if (el) { el.focus(); placeCaretEnd(el); }
}
function placeCaretEnd(el) {
  const r = document.createRange(); r.selectNodeContents(el); r.collapse(false);
  const s = window.getSelection(); s.removeAllRanges(); s.addRange(r);
}

// ---------- text editing ----------
function wireText(text, b, row, parentId) {
  text.addEventListener('input', () => {
    if (!b) return;
    debounce('blk-' + b.id, () => saveBlock(b.id, {}));
    const plain = text.textContent;
    if (plain.startsWith('/') && !slashOpen) openSlash(text, b);
    else if (slashOpen) filterSlash(plain.replace(/^\//, ''));
  });
  text.addEventListener('keydown', e => onBlockKey(e, text, b, parentId, row));
  text.addEventListener('blur', () => {
    if (!b && text.textContent.trim()) createFromEmpty(text, parentId);
  });
}

async function createFromEmpty(text, parentId) {
  if (!state.current) return;
  const { id } = await api.createBlock(state.current, { type: 'paragraph', parent_id: parentId, content: { text: text.textContent, html: text.innerHTML } });
  await reopen();
  focusBlock(id);
}

async function onBlockKey(e, text, b, parentId, row) {
  if (slashOpen) {
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') { e.preventDefault(); moveSlashSel(e.key === 'ArrowDown' ? 1 : -1); return; }
    if (e.key === 'Enter') { e.preventDefault(); pickSlashSel(text, b); return; }
    if (e.key === 'Escape') { closeSlash(); return; }
  }
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault();
    const after = await api.createBlock(state.current, { type: 'paragraph', parent_id: parentId, content: {} });
    await reopen();
    focusBlock(after.id);
  }
  if (e.key === 'Tab') {
    e.preventDefault();
    if (b) await indentBlock(b, e.shiftKey, row);
  }
  if (e.key === 'Backspace' && b && text.textContent === '') {
    e.preventDefault();
    await api.deleteBlock(b.id);
    reopen();
  }
  if ((e.metaKey || e.ctrlKey) && b) {
    const map = { b: 'bold', i: 'italic', u: 'underline' };
    const cmd = map[e.key.toLowerCase()];
    if (cmd) { e.preventDefault(); document.execCommand(cmd); saveBlock(b.id, {}); }
  }
}

async function indentBlock(b, outdent, row) {
  if (outdent) {
    if (!b.parent_id) return;
    const parentWrap = document.querySelector('.blockwrap[data-id="' + b.parent_id + '"]');
    const gpWrap = parentWrap && parentWrap.parentElement ? parentWrap.parentElement.closest('.blockwrap') : null;
    const newParent = gpWrap ? gpWrap.dataset.id : null;
    const r = await api.moveBlock(b.id, { parent_id: newParent });
    if (r.ok) reopen();
    return;
  }
  const wrap = row.closest('.blockwrap');
  const prev = wrap.previousElementSibling;
  if (!prev || !prev.classList.contains('blockwrap') || !prev.dataset.id) return;
  const r = await api.moveBlock(b.id, { parent_id: prev.dataset.id });
  if (r.ok) reopen(); else alert('Cannot nest there.');
}

// ---------- slash menu ----------
let slashOpen = false, slashSel = 0, slashFor = null;
function showSlashAt(b, parentId) { /* plus-button entry: create then convert */ }
function openSlash(text, b) {
  slashOpen = true; slashSel = 0; slashFor = { text, b };
  filterSlash(text.textContent.replace(/^\//, ''));
}
function closeSlash() { slashOpen = false; $('#slash-menu').hidden = true; }
function filterSlash(q) {
  const menu = $('#slash-menu');
  const items = SLASH.filter(s => s.name.toLowerCase().includes(q.toLowerCase()));
  if (!items.length) { closeSlash(); return; }
  menu.innerHTML = '';
  items.forEach((s, i) => {
    const mi = document.createElement('div');
    mi.className = 'mi' + (i === slashSel ? ' sel' : '');
    mi.innerHTML = '<span class="ic">' + s.ic + '</span><span class="lbl">' + s.name + '<small>' + s.desc + '</small></span>';
    mi.onclick = () => applySlash(s, slashFor.text, slashFor.b);
    menu.appendChild(mi);
  });
  const r = slashFor.text.getBoundingClientRect();
  menu.style.left = r.left + 'px';
  menu.style.top = (r.bottom + 4) + 'px';
  menu.hidden = false;
}
function moveSlashSel(d) {
  const items = $$('#slash-menu .mi');
  slashSel = (slashSel + d + items.length) % items.length;
  items.forEach((el, i) => el.classList.toggle('sel', i === slashSel));
}
function pickSlashSel(text, b) {
  const items = $$('#slash-menu .mi');
  const q = text.textContent.replace(/^\//, '');
  const matches = SLASH.filter(s => s.name.toLowerCase().includes(q.toLowerCase()));
  if (matches[slashSel]) applySlash(matches[slashSel], text, b);
}
async function applySlash(s, text, b) {
  closeSlash();
  if (!b) { await createFromEmpty(text, null); return; }
  text.textContent = '';
  if (s.type === 'divider') {
    await api.updateBlock(b.id, { content: { text: '' } });
    await fetch('/api/blocks/block/' + b.id, { method: 'PATCH', headers: J, body: JSON.stringify({ type: s.type, content: {} }) });
  } else {
    await fetch('/api/blocks/block/' + b.id, { method: 'PATCH', headers: J, body: JSON.stringify({ type: s.type }) });
  }
  await reopen();
  focusBlock(b.id);
}

// ---------- format toolbar ----------
document.addEventListener('mouseup', () => {
  setTimeout(() => {
    const sel = window.getSelection();
    const bar = $('#format-bar');
    if (sel && !sel.isCollapsed && sel.anchorNode && sel.anchorNode.parentElement && sel.anchorNode.parentElement.closest('.text')) {
      const r = sel.getRangeAt(0).getBoundingClientRect();
      bar.style.left = Math.max(8, r.left + r.width / 2 - 100) + 'px';
      bar.style.top = (r.top - 42) + 'px';
      bar.hidden = false;
    } else bar.hidden = true;
  }, 10);
});
$$('#format-bar button').forEach(btn => {
  btn.onmousedown = e => { e.preventDefault(); document.execCommand(btn.dataset.fmt); };
});

document.addEventListener('click', e => {
  if (!e.target.closest('#slash-menu') && !e.target.closest('.text')) closeSlash();
});

// ---------- databases ----------
const PROP_TYPES = [
  { type: 'text', name: 'Text' },
  { type: 'number', name: 'Number' },
  { type: 'select', name: 'Select' },
  { type: 'checkbox', name: 'Checkbox' },
  { type: 'date', name: 'Date' },
];
const SELECT_COLORS = 6;
let dbState = { db: null, rows: [], view: 'table', sortBy: null };

async function openDb(id) {
  state.currentDb = id;
  state.current = null;
  dbState.db = await api.getDb(id);
  dbState.rows = await api.rows(id);
  $('#page-view').hidden = true;
  $('#db-view').hidden = false;
  $('#db-title').value = dbState.db.name;
  $$('.view-tab').forEach(t => t.classList.toggle('active', t.dataset.view === dbState.view));
  renderDbView();
  renderTree();
}

$$('.view-tab').forEach(t => {
  t.onclick = () => { dbState.view = t.dataset.view; $$('.view-tab').forEach(x => x.classList.toggle('active', x === t)); renderDbView(); };
});
$('#db-title').addEventListener('input', () => {
  debounce('dbt', async () => {
    // name lives on data_source; reuse create-less update via a small PATCH-less trick:
    // databases don't have an update endpoint yet — refresh sidebar after name edit is applied on next load.
  });
});

function renderDbView() {
  const el = $('#db-body');
  el.innerHTML = '';
  if (dbState.view === 'board') renderBoard(el);
  else if (dbState.view === 'list') renderList(el);
  else renderTable(el);
}

function propDefs() { return dbState.db.properties || []; }
function propById(id) { return propDefs().find(p => p.id === id); }

function renderTable(el) {
  const t = document.createElement('table');
  t.className = 'db-table';
  const head = document.createElement('tr');
  propDefs().forEach(p => {
    const th = document.createElement('th');
    th.textContent = p.name;
    th.style.cursor = 'pointer';
    th.title = 'Click to sort';
    th.onclick = () => { dbState.sortBy = dbState.sortBy === p.id ? null : p.id; renderDbView(); };
    head.appendChild(th);
  });
  const addTh = document.createElement('th');
  addTh.textContent = '+';
  addTh.style.cursor = 'pointer';
  addTh.onclick = showPropMenu;
  head.appendChild(addTh);
  t.appendChild(head);

  let rows = dbState.rows.slice();
  if (dbState.sortBy) {
    const sp = propById(dbState.sortBy);
    rows.sort((a, b) => {
      const va = propVal(a, sp), vb = propVal(b, sp);
      return String(va == null ? '' : va).localeCompare(String(vb == null ? '' : vb));
    });
  }
  for (const row of rows) {
    const tr = document.createElement('tr');
    propDefs().forEach((p, i) => {
      const td = document.createElement('td');
      if (i === 0) {
        const open = document.createElement('span');
        open.className = 'row-open'; open.textContent = '⤢ ';
        open.title = 'Open as page';
        open.onclick = () => openPage(row.id);
        td.appendChild(open);
      }
      td.appendChild(cellEditor(row, p));
      tr.appendChild(td);
    });
    tr.appendChild(document.createElement('td'));
    t.appendChild(tr);
  }
  const addTr = document.createElement('tr');
  addTr.className = 'add-row';
  const addTd = document.createElement('td');
  addTd.colSpan = propDefs().length + 1;
  addTd.textContent = '+ New row';
  addTd.onclick = async () => { await api.createRow(dbState.db.id, {}); dbState.rows = await api.rows(dbState.db.id); renderDbView(); };
  addTr.appendChild(addTd);
  t.appendChild(addTr);
  el.appendChild(t);
}

function propVal(row, p) {
  if (!p) return null;
  if (p.type === 'title') return row.title;
  return row.props[p.id];
}

function cellEditor(row, p) {
  const v = propVal(row, p);
  if (p.type === 'title') {
    const inp = document.createElement('input');
    inp.value = row.title || '';
    inp.placeholder = 'Untitled';
    inp.onchange = async () => { await api.updatePage(row.id, { title: inp.value }); refreshSidebar(); };
    return inp;
  }
  if (p.type === 'checkbox') {
    const cb = document.createElement('input');
    cb.type = 'checkbox'; cb.checked = !!v;
    cb.onchange = async () => { await api.setProp(dbState.db.id, row.id, p.id, cb.checked); row.props[p.id] = cb.checked; };
    return cb;
  }
  if (p.type === 'select') {
    const sel = document.createElement('span');
    const idx = v == null ? -1 : (typeof v === 'number' ? v : 0);
    sel.className = 'cell-select c-' + Math.max(0, idx % SELECT_COLORS);
    sel.textContent = v == null ? 'Empty' : String(v);
    sel.onclick = e => showSelectMenu(e, row, p, sel);
    return sel;
  }
  const inp = document.createElement('input');
  inp.type = p.type === 'number' ? 'number' : (p.type === 'date' ? 'date' : 'text');
  inp.value = v == null ? '' : v;
  inp.onchange = async () => {
    const val = p.type === 'number' ? Number(inp.value) : inp.value;
    await api.setProp(dbState.db.id, row.id, p.id, val);
    row.props[p.id] = val;
  };
  return inp;
}

function showSelectMenu(e, row, p, selEl) {
  e.stopPropagation();
  const menu = $('#cell-menu');
  menu.innerHTML = '';
  const options = (p.options && p.options.length ? p.options : ['Todo', 'In progress', 'Done']);
  options.forEach((opt, i) => {
    const mi = document.createElement('div');
    mi.className = 'mi';
    mi.innerHTML = '<span class="cell-select c-' + (i % SELECT_COLORS) + '">' + opt + '</span>';
    mi.onclick = async () => {
      menu.hidden = true;
      await api.setProp(dbState.db.id, row.id, p.id, opt);
      row.props[p.id] = opt;
      dbState.rows = await api.rows(dbState.db.id);
      renderDbView();
    };
    menu.appendChild(mi);
  });
  const clear = document.createElement('div');
  clear.className = 'mi'; clear.innerHTML = '<span style="color:var(--muted)">Clear</span>';
  clear.onclick = async () => { menu.hidden = true; await api.setProp(dbState.db.id, row.id, p.id, null); delete row.props[p.id]; renderDbView(); };
  menu.appendChild(clear);
  const r = selEl.getBoundingClientRect();
  menu.style.left = r.left + 'px'; menu.style.top = (r.bottom + 4) + 'px';
  menu.hidden = false;
}

function showPropMenu(e) {
  e.stopPropagation();
  const menu = $('#prop-menu');
  menu.innerHTML = '';
  PROP_TYPES.forEach(pt => {
    const mi = document.createElement('div');
    mi.className = 'mi';
    mi.innerHTML = '<span class="ic">' + pt.name[0] + '</span><span class="lbl">' + pt.name + '</span>';
    mi.onclick = async () => {
      menu.hidden = true;
      await api.addProp(dbState.db.id, { name: pt.name, type: pt.type });
      dbState.db = await api.getDb(dbState.db.id);
      renderDbView();
    };
    menu.appendChild(mi);
  });
  const r = e.target.getBoundingClientRect();
  menu.style.left = (r.left - 180) + 'px'; menu.style.top = (r.bottom + 4) + 'px';
  menu.hidden = false;
}
document.addEventListener('click', e => {
  if (!e.target.closest('#cell-menu')) $('#cell-menu').hidden = true;
  if (!e.target.closest('#prop-menu')) $('#prop-menu').hidden = true;
});

function renderBoard(el) {
  const statusProp = propDefs().find(p => p.type === 'select');
  const board = document.createElement('div');
  board.className = 'db-board';
  const cols = statusProp && statusProp.options && statusProp.options.length
    ? statusProp.options : ['Todo', 'In progress', 'Done'];
  cols.forEach(colName => {
    const col = document.createElement('div');
    col.className = 'board-col';
    const h = document.createElement('h4');
    h.textContent = colName;
    col.appendChild(h);
    const inCol = dbState.rows.filter(r => statusProp ? r.props[statusProp.id] === colName : false);
    inCol.forEach(r => {
      const card = document.createElement('div');
      card.className = 'board-card';
      card.textContent = r.title || 'Untitled';
      card.onclick = () => openPage(r.id);
      col.appendChild(card);
    });
    const add = document.createElement('div');
    add.className = 'add-card'; add.textContent = '+ New';
    add.onclick = async () => {
      const { id } = await api.createRow(dbState.db.id, {});
      if (statusProp) await api.setProp(dbState.db.id, id, statusProp.id, colName);
      dbState.rows = await api.rows(dbState.db.id);
      renderDbView();
    };
    col.appendChild(add);
    board.appendChild(col);
  });
  // "No status" column
  const none = document.createElement('div');
  none.className = 'board-col';
  none.innerHTML = '<h4>No status</h4>';
  dbState.rows.filter(r => !statusProp || r.props[statusProp.id] == null).forEach(r => {
    const card = document.createElement('div');
    card.className = 'board-card';
    card.textContent = r.title || 'Untitled';
    card.onclick = () => openPage(r.id);
    none.appendChild(card);
  });
  board.appendChild(none);
  el.appendChild(board);
}

function renderList(el) {
  const c = document.createElement('div');
  c.className = 'db-list';
  dbState.rows.forEach(r => {
    const li = document.createElement('div');
    li.className = 'li';
    const t = document.createElement('span');
    t.textContent = (r.icon || '📄') + ' ' + (r.title || 'Untitled');
    li.appendChild(t);
    const pr = document.createElement('span');
    pr.className = 'props';
    pr.textContent = propDefs().filter(p => p.type !== 'title').map(p => {
      const v = propVal(r, p);
      return v == null || v === '' ? '' : p.name + ': ' + v;
    }).filter(Boolean).join(' · ');
    li.appendChild(pr);
    li.onclick = () => openPage(r.id);
    c.appendChild(li);
  });
  const add = document.createElement('div');
  add.className = 'li'; add.style.color = 'var(--muted)'; add.textContent = '+ New row';
  add.onclick = async () => { await api.createRow(dbState.db.id, {}); dbState.rows = await api.rows(dbState.db.id); renderDbView(); };
  c.appendChild(add);
  el.appendChild(c);
}

// ---------- boot ----------
(async () => {
  await refreshSidebar();
  const first = state.pages[0];
  if (first) openPage(first.id);
  else {
    const { id } = await api.createPage({ title: 'Getting started' });
    await refreshSidebar();
    openPage(id);
  }
})();
