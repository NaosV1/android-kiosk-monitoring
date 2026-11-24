'use strict';
/* Interface tactile du monitoring (1280x800 logiques = ecran 1920x1200 a x1,5). */

const app = document.getElementById('app');
const S = {
  etat: null, erreur: false,
  service: null, serviceAt: 0,
  incidents: null, incidentsAt: 0,
  filtre: 'tous',
  derniereAction: Date.now(),
  ouvertsConnus: null,
  doigt: false, renduEnAttente: false,
  toast: null,
  confirmerMaj: null,        // version en attente de confirmation (fenetre ouverte)
  majEnvoyee: null,          // version dont l'installation a ete demandee
  toastMajVu: false,
};

// ------------------------------------------------------------------ outils
const HEX = { up: '#3DD68C', warn: '#F5A524', lent: '#F5A524', down: '#FF5C5C', inconnu: '#93A1B5' };
const SERIES = ['#6CB6FF', '#C4A5FF', '#FF8CC6', '#7EE0C3', '#F2CC60', '#A0B4FF', '#FFB38A', '#9BE15D'];
const RANG = { down: 0, warn: 1, inconnu: 2, up: 3 };

const esc = (s) => String(s ?? '').replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
const maintenant = () => Date.now() / 1000;

function duree(s) {
  s = Math.max(0, Math.round(s));
  if (s < 60) return `${s} s`;
  const m = Math.floor(s / 60);
  if (m < 60) return `${m} min`;
  const h = Math.floor(m / 60), r = m % 60;
  if (h < 24) return r ? `${h} h ${String(r).padStart(2, '0')}` : `${h} h`;
  const j = Math.floor(h / 24);
  return `${j} j`;
}
const ilya = (ts) => (ts ? (maintenant() - ts < 45 ? "à l'instant" : `il y a ${duree(maintenant() - ts)}`) : 'jamais');
const heure = (ts) => new Date(ts * 1000).toLocaleTimeString('fr-FR', { hour: '2-digit', minute: '2-digit' });
const jourLong = (d) => d.toLocaleDateString('fr-FR', { weekday: 'long', day: 'numeric', month: 'long' });
const memeJour = (a, b) => a.toDateString() === b.toDateString();
function quand(ts) {
  const d = new Date(ts * 1000);
  if (memeJour(d, new Date())) return heure(ts);
  return `${String(d.getDate()).padStart(2, '0')}/${String(d.getMonth() + 1).padStart(2, '0')} ${heure(ts)}`;
}
const pct = (v) => (v == null ? '—' : `${v >= 99.995 ? '100' : v.toFixed(2).replace('.', ',')} %`);
const ms = (v) => (v == null ? '—' : `${Math.round(v).toLocaleString('fr-FR')} ms`);
const pluriel = (n, mot, motP) => `${n} ${n > 1 ? (motP || mot + 's') : mot}`;

function svg(contenu, taille = 26, epaisseur = 1.8, couleur = 'currentColor') {
  return `<svg width="${taille}" height="${taille}" viewBox="0 0 24 24" fill="none" stroke="${couleur}" stroke-width="${epaisseur}" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${contenu}</svg>`;
}
const D = {
  ok: '<path d="M5 12.5l4.5 4.5L19 7.5"/>',
  warn: '<path d="M12 3.5L22 20.5H2z"/><path d="M12 10v4.5"/><path d="M12 17.5v.01"/>',
  warnPetit: '<path d="M12 3.5L22 20.5H2z"/><path d="M12 10v4"/>',
  down: '<path d="M6 6l12 12M18 6L6 18"/>',
  attente: '<path d="M6 12h.01M12 12h.01M18 12h.01"/>',
  accueil: '<rect x="3.5" y="3.5" width="7" height="7" rx="1.5"/><rect x="13.5" y="3.5" width="7" height="7" rx="1.5"/><rect x="3.5" y="13.5" width="7" height="7" rx="1.5"/><rect x="13.5" y="13.5" width="7" height="7" rx="1.5"/>',
  tailscale: '<circle cx="6" cy="12" r="2.5"/><circle cx="18" cy="6" r="2.5"/><circle cx="18" cy="18" r="2.5"/><path d="M8.2 10.9l7.6-3.8M8.2 13.1l7.6 3.8"/>',
  cloche: '<path d="M6 16v-5a6 6 0 0 1 12 0v5l1.5 2h-15z"/><path d="M10 20.5a2 2 0 0 0 4 0"/>',
  lune: '<path d="M20 14.5A8 8 0 0 1 9.5 4a8 8 0 1 0 10.5 10.5z"/>',
  pouls: '<path d="M3 12h4l2.5-6 5 12 2.5-6h4"/>',
  gauche: '<path d="M15 6l-6 6 6 6"/>',
  droite: '<path d="M9 6l6 6-6 6"/>',
  relancer: '<path d="M20 11a8 8 0 1 0-2.3 5.7"/><path d="M20 5v6h-6"/>',
  info: '<circle cx="12" cy="12" r="9"/><path d="M12 11v5"/><path d="M12 8v.01"/>',
  serveur: '<rect x="3.5" y="4" width="17" height="7" rx="1.5"/><rect x="3.5" y="13" width="17" height="7" rx="1.5"/><path d="M7.5 7.5h.01M7.5 16.5h.01"/>',
  tablette: '<rect x="4.5" y="3" width="15" height="18" rx="2"/><path d="M11 18h2"/>',
  ordi: '<rect x="3" y="4" width="18" height="12" rx="1.5"/><path d="M8 20h8M12 16v4"/>',
  tel: '<rect x="7" y="2.5" width="10" height="19" rx="2"/><path d="M11 18.5h2"/>',
  maj: '<path d="M12 3.5v11"/><path d="M7.5 10l4.5 4.5 4.5-4.5"/><path d="M4.5 19.5h15"/>',
};
function iconeEtat(etat, taille = 16) {
  const c = HEX[etat] || HEX.inconnu;
  if (etat === 'up') return svg(D.ok, taille, taille > 20 ? 2.4 : 3, c);
  if (etat === 'warn' || etat === 'lent') return svg(taille > 20 ? D.warn : D.warnPetit, taille, taille > 20 ? 2.2 : 2.6, c);
  if (etat === 'down') return svg(D.down, taille, taille > 20 ? 2.4 : 3, c);
  return svg(D.attente, taille, 3.5, c);
}

// ------------------------------------------------------------------ calculs
function problemes() {
  const e = S.etat, liste = [];
  for (const s of e.services) {
    if (s.etat !== 'warn' && s.etat !== 'down') continue;
    liste.push({
      niveau: s.etat,
      titre: s.etat === 'down' ? `${s.nom} est hors ligne` : `${s.nom} est dégradé`,
      court: `${s.nom} ${s.etat === 'down' ? 'hors ligne' : 'dégradé'}`,
      detail: s.message, depuis: s.depuis, href: `#/service/${encodeURIComponent(s.id)}`,
    });
  }
  const t = e.tailscale;
  if (t.actif && !t.tablette.ip) {
    liste.push({ niveau: 'down', titre: 'Tailscale déconnecté', court: 'Tailscale déconnecté',
      detail: "La tablette n'est plus sur le tailnet", depuis: t.tablette.depuis, href: '#/tailscale' });
  }
  if (t.actif && t.exit_node && t.exit_node.actif) {
    liste.push({ niveau: 'warn', titre: 'Exit node activé sur la tablette', court: 'Exit node activé',
      detail: "Les contrôles Internet passent par l'entreprise", depuis: t.exit_node.depuis, href: '#/tailscale' });
  }
  return liste;
}
const pire = (liste) => (liste.some((p) => p.niveau === 'down') ? 'down' : 'warn');
const nodesEnLigne = (s) => s.nodes.filter((n) => n.etat === 'up' || n.etat === 'lent').length;
function compteNodes(s) {
  const total = s.nodes.length, ok = nodesEnLigne(s);
  if (s.etat === 'inconnu' || ok === total) return pluriel(total, 'node');
  return `${ok}/${total} ${total > 1 ? 'nodes' : 'node'}`;
}

// ------------------------------------------------------------------ morceaux communs
function rail(page) {
  const ouverts = S.etat ? S.etat.incidents_ouverts : [];
  const lien = (href, cle, libelle, icone, extra = '') =>
    `<a href="${href}" class="rail-lien appui${page === cle ? ' actif' : ''}"${page === cle ? ' aria-current="page"' : ''}>${svg(icone)}<span>${libelle}</span>${extra}</a>`;
  const badge = ouverts.length ? `<span class="pastille ${pire(ouverts)}">${ouverts.length}</span>` : '';
  const m = S.etat && S.etat.maj;
  const badgeMaj = m && m.disponible && (m.statut === 'prete' || m.statut === 'telechargement') ? '<span class="pastille info">1</span>' : '';
  return `<nav class="rail" aria-label="Navigation principale">
    <div class="logo">${svg(D.pouls, 26, 2, '#6CB6FF')}</div>
    ${lien('#/', 'accueil', 'Accueil', D.accueil)}
    ${lien('#/tailscale', 'tailscale', 'Tailscale', D.tailscale)}
    ${lien('#/incidents', 'incidents', 'Incidents', D.cloche, badge)}
    ${lien('#/maj', 'maj', 'Mise à jour', D.maj, badgeMaj)}
    <a href="#/veille" class="rail-lien bas appui">${svg(D.lune)}<span>Veille</span></a>
  </nav>`;
}

function horloge() {
  const d = new Date();
  return `<div class="horloge"><span class="heure">${d.toLocaleTimeString('fr-FR', { hour: '2-digit', minute: '2-digit' })}</span><span class="date">${jourLong(d)}</span></div>`;
}

function piluleTailscale() {
  const t = S.etat.tailscale;
  let point, libelle, detail;
  if (t.tablette.ip && t.exit_node && t.exit_node.actif) {
    point = ' warn'; libelle = '<strong class="warn">exit node activé</strong>'; detail = `<span class="mono discret">${esc(t.tablette.ip)}</span>`;
  } else if (t.tablette.ip) {
    point = ''; libelle = '<strong class="up">connecté</strong>'; detail = `<span class="mono discret">${esc(t.tablette.ip)}</span>`;
  } else if (t.actif) {
    point = ' down'; libelle = '<strong class="down">déconnecté</strong>'; detail = `<span class="discret">depuis ${duree(maintenant() - t.tablette.depuis)}</span>`;
  } else {
    point = ' gris'; libelle = '<strong class="discret">pas sur le tailnet</strong>'; detail = '';
  }
  return `<a href="#/tailscale" class="pilule appui"><span class="point${point}"></span><span>Tailscale</span>${libelle}${detail}</a>`;
}

function sparkline(vals, etat) {
  const pts = vals || [];
  const def = pts.filter((v) => v != null);
  if (!def.length) {
    return '<svg class="spark" viewBox="0 0 200 34" preserveAspectRatio="none" aria-hidden="true"><line x1="0" y1="31" x2="200" y2="31" stroke="#263041" stroke-width="2" stroke-dasharray="4 4" vector-effect="non-scaling-stroke"/></svg>';
  }
  const max = Math.max(...def) * 1.25 || 1, n = Math.max(pts.length - 1, 1);
  const segs = []; let cur = [];
  pts.forEach((v, i) => {
    if (v == null) { if (cur.length) segs.push(cur); cur = []; return; }
    cur.push([i * 200 / n, 31 - (v / max) * 27]);
  });
  if (cur.length) segs.push(cur);
  const c = etat === 'up' ? '#4C8C6E' : etat === 'inconnu' ? '#4A5568' : HEX[etat];
  const lignes = segs.map((seg) => {
    if (seg.length === 1) seg = [[seg[0][0] - 1.5, seg[0][1]], [seg[0][0] + 1.5, seg[0][1]]];
    return `<polyline points="${seg.map((p) => `${p[0].toFixed(1)},${p[1].toFixed(1)}`).join(' ')}" fill="none" stroke="${c}" stroke-width="2" stroke-linejoin="round" stroke-linecap="round" vector-effect="non-scaling-stroke"/>`;
  }).join('');
  return `<svg class="spark" viewBox="0 0 200 34" preserveAspectRatio="none" aria-hidden="true">${lignes}</svg>`;
}

// ------------------------------------------------------------------ Accueil
function bandeau() {
  const e = S.etat;
  if (!e.services.length) {
    return `<section class="bandeau inconnu" aria-label="État général"><span class="icone">${svg(D.info, 26, 2, '#6CB6FF')}</span>
      <div class="textes"><strong>Aucun service configuré</strong><span>Ajoute tes services dans /root/monitoring/config.toml, puis lance kiosk-restart.</span></div></section>`;
  }
  const pbs = problemes();
  if (!pbs.length) {
    const tous = e.services.flatMap((s) => s.nodes);
    if (e.services.every((s) => s.etat === 'inconnu')) {
      return `<section class="bandeau inconnu" aria-label="État général"><span class="icone">${iconeEtat('inconnu', 22)}</span>
        <div class="textes"><strong>Premiers contrôles en cours…</strong><span>Les résultats arrivent dans quelques secondes.</span></div></section>`;
    }
    const dernier = Math.max(...tous.map((n) => n.dernier || 0));
    return `<section class="bandeau up" aria-label="État général"><span class="icone">${iconeEtat('up', 22)}</span>
      <div class="textes"><strong class="up">Tout est opérationnel</strong>
      <span>${pluriel(e.services.length, 'service')} et ${pluriel(tous.length, 'node')} répondent normalement · dernier contrôle ${ilya(dernier)}</span></div></section>`;
  }
  let niveau, titre, detail, href;
  if (pbs.length === 1) {
    const p = pbs[0];
    niveau = p.niveau; titre = p.titre; href = p.href;
    detail = `${p.detail} · depuis ${duree(maintenant() - p.depuis)}`;
  } else {
    niveau = pire(pbs); titre = `${pbs.length} problèmes en cours`; href = '#/incidents';
    detail = pbs.map((p) => p.court).join(' · ');
  }
  return `<section class="bandeau ${niveau}" aria-label="État général"><span class="icone">${iconeEtat(niveau, 22)}</span>
    <div class="textes"><strong class="${niveau}">${esc(titre)}</strong><span>${esc(detail)}</span></div>
    <a href="${href}" class="bouton plein ${niveau} appui"><span>Voir le détail</span>${svg(D.droite, 20, 2.2)}</a></section>`;
}

function tuile(s) {
  const k = s.etat;
  const nodes = s.nodes.map((n) => `<span class="node${n.etat === 'down' ? ' creux' : n.etat === 'lent' ? ' lent' : n.etat === 'inconnu' ? ' gris' : ''}"></span>`).join('');
  const latence = k === 'down' ? 'Hors ligne' : k === 'inconnu' ? '…' : ms(s.ms);
  let note;
  if (k === 'up') note = s.dispo30 != null ? pct(s.dispo30) : '';
  else if (k === 'inconnu') note = 'en attente';
  else note = `depuis ${duree(maintenant() - s.depuis)}`;
  return `<a href="#/service/${encodeURIComponent(s.id)}" class="tuile ${k} appui">
    <div class="ligne"><span class="badge ${k}">${iconeEtat(k)}</span><span class="nom${s.nom.length > 14 ? ' long' : ''}">${esc(s.nom)}</span><span class="latence ${k}">${latence}</span></div>
    <span class="hote">${esc(s.adresse)}</span>
    <div class="ligne petite">${nodes}<span class="nodes-label">${compteNodes(s)}</span>${sparkline(s.spark, k)}<span class="note ${k}">${note}</span></div>
  </a>`;
}

// Regroupe les services par tag, dans l'ordre de la config ; les services sans tag vont a la fin.
function groupes(services) {
  const ordre = [], par = new Map();
  for (const s of services) {
    const t = s.tag || '';
    if (!par.has(t)) { par.set(t, []); ordre.push(t); }
    par.get(t).push(s);
  }
  ordre.sort((a, b) => (a === '') - (b === ''));
  return ordre.map((t) => ({ tag: t, services: par.get(t) }));
}

function enteteGroupe(nom, liste) {
  const hs = liste.filter((s) => s.etat === 'down').length;
  const deg = liste.filter((s) => s.etat === 'warn').length;
  const niveau = hs ? 'down' : deg ? 'warn' : liste.every((s) => s.etat === 'inconnu') ? 'inconnu' : 'up';
  const morceaux = [];
  if (hs) morceaux.push(`<span class="down">${hs} hors ligne</span>`);
  if (deg) morceaux.push(`<span class="warn">${pluriel(deg, 'dégradé')}</span>`);
  const resume = morceaux.length ? `${morceaux.join(' · ')} · ${liste.length} au total` : pluriel(liste.length, 'service');
  return `<div class="groupe-titre"><span class="pt ${niveau}"></span><h2>${esc(nom)}</h2><span class="resume-groupe">${resume}</span><span class="trait"></span></div>`;
}

function pageAccueil() {
  const e = S.etat;
  const tous = e.services.flatMap((s) => s.nodes);
  const nodesOk = tous.filter((n) => n.etat === 'up' || n.etat === 'lent').length;
  const servicesOk = e.services.filter((s) => s.etat === 'up').length;
  const gs = groupes(e.services);
  const titres = gs.length > 1 || (gs.length === 1 && gs[0].tag);
  // dans chaque groupe, les problemes passent en premier (sinon ordre de la config)
  const contenu = e.services.length
    ? `<div class="groupes">${gs.map((g) => `<section class="groupe" aria-label="${esc(g.tag || 'Services')}">
        ${titres ? enteteGroupe(g.tag || 'Autres', g.services) : ''}
        <div class="grille">${[...g.services].sort((a, b) => RANG[a.etat] - RANG[b.etat]).map(tuile).join('')}</div></section>`).join('')}</div>`
    : `<div class="vide">Aucun service pour l'instant. Ouvre <code>/root/monitoring/config.toml</code> en SSH (port 2222), ajoute tes services, puis lance <code>kiosk-restart</code>.</div>`;
  return `<div class="ecran">${rail('accueil')}<main class="contenu accueil">
    <header class="entete"><div class="titres"><h1>Vue d'ensemble</h1>
      <p class="sous">${servicesOk}/${e.services.length} services opérationnels · ${nodesOk}/${tous.length} nodes en ligne · contrôle toutes les ${e.intervalle} s</p></div>
      ${piluleTailscale()}${horloge()}</header>
    ${bandeau()}
    <div class="defile" data-defile="accueil">${contenu}</div>
  </main></div>`;
}

// ------------------------------------------------------------------ Detail d'un service
function plafond(v) {
  for (const p of [10, 20, 25, 50, 100, 200, 250, 400, 500, 1000, 2000, 2500, 5000, 10000, 20000, 60000]) if (v <= p) return p;
  return Math.ceil(v / 10000) * 10000;
}

function graphe(s, d) {
  if (!d) return '<p class="discret">Chargement…</p>';
  const series = s.nodes.map((n, i) => ({ n, c: SERIES[i % SERIES.length], v: d.courbes[n.id] || [] }));
  const valeurs = series.flatMap((x) => x.v.filter((v) => v != null));
  if (!valeurs.length) return '<p class="discret" style="padding:40px 0;text-align:center">Pas encore de mesures : la courbe se remplit au fil des contrôles.</p>';
  const seuil = S.etat.seuil_lent;
  const max = plafond(Math.max(...valeurs) * 1.2);
  const x0 = 60, x1 = 632, yb = 170, yh = 10;
  const nb = Math.max(...series.map((x) => x.v.length)) - 1 || 1;
  const X = (i) => x0 + (i * (x1 - x0)) / nb;
  const Y = (v) => yb - (Math.min(v, max) / max) * (yb - yh);
  const fmt = (v) => (v >= 1000 ? `${(v / 1000).toLocaleString('fr-FR')} s` : `${v} ms`);
  let out = `<svg class="graphe" viewBox="0 0 640 186" role="img" aria-label="Temps de réponse par node sur 24 heures">`;
  out += `<g stroke="#1E2733" stroke-width="1"><line x1="${x0}" y1="${yb}" x2="636" y2="${yb}"/><line x1="${x0}" y1="${Y(max / 2)}" x2="636" y2="${Y(max / 2)}"/><line x1="${x0}" y1="${yh}" x2="636" y2="${yh}"/></g>`;
  out += `<g fill="#93A1B5" font-size="12" font-family="Inter, sans-serif" text-anchor="end"><text x="52" y="${yb + 4}">0</text><text x="52" y="${Y(max / 2) + 4}">${fmt(max / 2)}</text><text x="52" y="${yh + 4}">${fmt(max)}</text></g>`;
  if (seuil < max) {
    out += `<line x1="${x0}" y1="${Y(seuil)}" x2="636" y2="${Y(seuil)}" stroke="#7A5716" stroke-width="1.5" stroke-dasharray="6 5"/>`;
    out += `<text x="${x0 + 4}" y="${Y(seuil) - 6}" fill="#F5A524" font-size="12" font-family="Inter, sans-serif">seuil « lent » ${fmt(seuil)}</text>`;
  }
  for (const sr of series) {
    let seg = [];
    const tracer = () => {
      if (seg.length === 1) seg.push([seg[0][0] + 2, seg[0][1]]);
      if (seg.length) out += `<polyline points="${seg.map((p) => `${p[0].toFixed(1)},${p[1].toFixed(1)}`).join(' ')}" fill="none" stroke="${sr.c}" stroke-width="2" stroke-linejoin="round" stroke-linecap="round"/>`;
      seg = [];
    };
    sr.v.forEach((v, i) => { if (v == null) tracer(); else seg.push([X(i), Y(v)]); });
    tracer();
    if (sr.n.etat === 'down') {
      let k = sr.v.length - 1;
      while (k >= 0 && sr.v[k] == null) k -= 1;
      if (k >= 0) {
        const ex = X(k), ey = Y(sr.v[k]);
        out += `<circle cx="${ex}" cy="${ey}" r="7" fill="#251416" stroke="#FF5C5C" stroke-width="2"/><path d="M${ex - 3.5} ${ey - 3.5}L${ex + 3.5} ${ey + 3.5}M${ex + 3.5} ${ey - 3.5}L${ex - 3.5} ${ey + 3.5}" stroke="#FF5C5C" stroke-width="2" stroke-linecap="round"/>`;
      }
    }
  }
  out += '</svg><div class="axe-x"><span>il y a 24 h</span><span>18 h</span><span>12 h</span><span>6 h</span><span>maintenant</span></div>';
  return out;
}

function carteNode(n, couleur) {
  const k = n.etat;
  const serie = `<span class="serie" style="background:${couleur}"></span>`;
  if (k === 'down') {
    return `<article class="carte-node down"><div class="haut"><span class="badge down">${iconeEtat('down', 18)}</span>
      <div class="ident"><strong>${serie}${esc(n.nom)}</strong><span>${esc(n.lieu || n.type.toUpperCase())}</span></div>
      <div class="mesure"><b>Hors ligne</b><span>depuis ${duree(maintenant() - n.depuis)}</span></div></div>
      <p>${esc(n.info)} · dernier contrôle ${ilya(n.dernier)}</p>
      <div class="bas"><span class="mono">${esc(n.cible)}</span></div></article>`;
  }
  const mesure = k === 'inconnu' ? '…' : ms(n.ms);
  return `<article class="carte-node${k === 'lent' ? ' lent' : ''}"><div class="haut"><span class="badge ${k}">${iconeEtat(k, 18)}</span>
    <div class="ident"><strong>${serie}${esc(n.nom)}</strong><span>${esc(n.lieu || n.type.toUpperCase())}</span></div>
    <div class="mesure"><b class="${k === 'lent' ? 'warn' : ''}">${mesure}</b><span>${esc(k === 'lent' ? `${n.info} · lent` : n.info)}</span></div></div>
    <div class="bas"><span class="mono">${esc(n.cible)}</span><span class="droite">contrôlé ${ilya(n.dernier)}</span></div></article>`;
}

function pageService(id) {
  const e = S.etat;
  const s = e.services.find((x) => x.id === id);
  if (!s) return pageAccueil();
  const d = S.service && S.service.id === id ? S.service : null;
  const ok = nodesEnLigne(s), total = s.nodes.length;
  const libelles = { up: 'Opérationnel', warn: total > 1 ? `Dégradé · ${ok}/${total} nodes` : 'Dégradé', down: 'Hors ligne', inconnu: 'En attente' };
  const couleurs = new Map(s.nodes.map((n, i) => [n.id, SERIES[i % SERIES.length]]));
  const ordre = { down: 0, lent: 1, inconnu: 2, up: 3 };
  const nodes = [...s.nodes].sort((a, b) => ordre[a.etat] - ordre[b.etat]);
  const enLigne = s.nodes.filter((n) => n.etat === 'up' || n.etat === 'lent').map((n) => n.nom);
  const note = s.etat === 'warn' && total > 1 && enLigne.length
    ? `<div class="note-info">${svg(D.info, 20, 2, '#6CB6FF')}<span>Le service reste joignable par ${esc(enLigne.join(' et '))}. Il passera « hors ligne » seulement si plus aucune node ne répond.</span></div>` : '';
  const legende = s.nodes.map((n) => `<span><span class="serie" style="background:${couleurs.get(n.id)}"></span>${esc(n.nom)}</span>`).join('');
  const jours = d ? d.jours.map((j) => `<span class="${j === 'up' ? '' : j === 'vide' ? 'nd' : j}"></span>`).join('') : '';
  const evenements = d && d.evenements.length
    ? d.evenements.slice(0, 6).map((i) => `<div class="evenement"><span class="quand">${quand(i.debut)}</span><span class="marque ${i.fin ? 'up' : i.niveau}"></span>
        <span class="quoi">${esc(i.message)} · ${i.fin ? `résolu après ${duree(i.fin - i.debut)}` : 'en cours'}</span></div>`).join('')
    : '<p class="discret" style="font-size:15px">Aucun incident enregistré.</p>';
  const cible = total === 1 ? `<span class="mono">${esc(s.nodes[0].cible)}</span>` : esc(s.adresse);
  return `<div class="ecran">${rail('accueil')}<main class="contenu">
    <header class="entete">
      <a href="#/" class="retour appui" aria-label="Retour à l'accueil">${svg(D.gauche, 26, 2.2)}</a>
      <div class="titres"><div class="ligne-titre"><h1>${esc(s.nom)}</h1><span class="etiquette ${s.etat} ${s.etat}">${iconeEtat(s.etat)}<span>${libelles[s.etat]}</span></span></div>
        <p class="sous">${cible} · contrôle toutes les ${e.intervalle} s</p></div>
      <button type="button" class="bouton appui" data-action="relancer" data-id="${esc(s.id)}">${svg(D.relancer, 22, 2)}<span>Relancer les contrôles</span></button>
    </header>
    <div class="deux-colonnes" style="grid-template-columns: minmax(0, 5fr) minmax(0, 7fr);">
      <section class="colonne defile" data-defile="nodes" aria-label="Nodes">
        <div class="titre-section"><h2>${total > 1 ? 'Nodes' : 'Node'}</h2><span>${ok} sur ${total} en ligne</span></div>
        ${nodes.map((n) => carteNode(n, couleurs.get(n.id))).join('')}
        ${note}
      </section>
      <div class="colonne">
        <section class="carte" aria-label="Temps de réponse sur 24 heures"><div class="legende"><h2>Temps de réponse · 24 h</h2>${legende}</div>${graphe(s, d)}</section>
        <section class="carte" aria-label="Disponibilité sur 90 jours">
          <div class="legende"><h2>Disponibilité · 90 jours</h2><span style="font-size:20px;font-weight:700;color:#E8EDF4">${d ? pct(d.dispo90) : '—'}</span></div>
          <div class="barres" aria-hidden="true">${jours}</div>
          <div class="sous-barres"><span>il y a 90 jours</span><span>${d ? pluriel(d.jours_avec_incident, 'jour') + ' avec incident' : ''}</span><span>aujourd'hui</span></div>
        </section>
        <section class="carte defile" data-defile="evenements" style="flex:1" aria-label="Derniers événements"><h2>Derniers événements</h2>${evenements}</section>
      </div>
    </div>
  </main></div>`;
}

// ------------------------------------------------------------------ Tailscale
function iconeAppareil(a) {
  if (a.cette_tablette) return D.tablette;
  const os = (a.os || '').toLowerCase();
  if (os.includes('android') || os.includes('ios')) return D.tel;
  if (os.includes('windows') || os.includes('macos')) return D.ordi;
  return D.serveur;
}

function pageTailscale() {
  const t = S.etat.tailscale, api = t.api;
  const ip = t.tablette.ip;
  const etiquette = ip ? '<span class="etiquette up up"><span class="point"></span><span>Connecté</span></span>'
    : t.actif ? '<span class="etiquette down down"><span class="point down"></span><span>Déconnecté</span></span>'
      : '<span class="etiquette inconnu inconnu"><span class="point gris"></span><span>Tablette hors tailnet</span></span>';
  const sous = api.tailnet ? `<span class="mono">${esc(api.tailnet)}</span> · liste mise à jour ${ilya(api.maj)}`
    : api.configuree ? 'Lecture de la liste des appareils…' : 'Liste des appareils : clé API non configurée';
  const appareils = api.appareils || [];
  const moi = appareils.find((a) => a.cette_tablette);
  const enLigne = appareils.filter((a) => a.en_ligne);
  const hs = appareils.filter((a) => !a.en_ligne);
  const recents = hs.filter((a) => a.vu && maintenant() - a.vu < 86400);
  const sorties = appareils.filter((a) => a.sortie).map((a) => a.nom);

  let corps;
  if (!api.configuree) {
    corps = `<section class="carte aide"><h2>Afficher tous les appareils du tailnet</h2>
      <span>1. Dans la console Tailscale, ouvre <b>Settings › Keys</b> et génère une clé API (<code>tskey-api-…</code>).</span>
      <span>2. En SSH sur la tablette (port 2222), ouvre <code>/root/monitoring/config.toml</code> et colle-la dans <code>cle_api = "…"</code>, section <code>[tailscale]</code>.</span>
      <span>3. Lance <code>kiosk-restart</code>. La liste apparaît ici en moins d'une minute.</span>
      <span class="discret">Une clé API expire au bout de 90 jours au maximum : il faudra la renouveler.</span></section>`;
  } else if (!api.appareils) {
    corps = '<section class="carte"><p class="discret">En attente de la première réponse de l\'API Tailscale…</p></section>';
  } else {
    const ordre = (a) => (!a.en_ligne && recents.includes(a) ? 0 : a.cette_tablette ? 1 : a.en_ligne ? 2 : 3);
    const tri = [...appareils].sort((a, b) => ordre(a) - ordre(b) || a.nom.localeCompare(b.nom));
    const lignes = tri.map((a) => {
      const alerte = !a.en_ligne && recents.includes(a);
      const puces = (a.cette_tablette ? '<span class="puce moi">cette tablette</span>' : '') +
        (a.sortie ? '<span class="puce">nœud de sortie</span>' : '') +
        (a.sous_reseaux.length ? '<span class="puce">routeur de sous-réseau</span>' : '');
      const etat = a.en_ligne ? '<span class="etat-ligne up up"><span class="point"></span>En ligne</span>'
        : alerte ? '<span class="etat-ligne down down"><span class="point down"></span>Hors ligne</span>'
          : '<span class="etat-ligne gris"><span class="point gris"></span>Hors ligne</span>';
      const vu = a.en_ligne ? 'maintenant' : a.vu ? ilya(a.vu) : '—';
      return `<tr class="${alerte ? 'alerte' : ''}">
        <td><div class="appareil"><span class="ic${a.cette_tablette ? ' moi' : alerte ? ' hs' : ''}">${svg(iconeAppareil(a), 20)}</span>
          <div><strong>${esc(a.nom)}${puces}</strong><small>${esc(a.os)}${a.sous_reseaux.length ? ' · ' + esc(a.sous_reseaux.join(', ')) : ''}</small></div></div></td>
        <td class="mono" style="font-size:14px;color:#C9D3E0">${esc(a.ip)}</td>
        <td style="color:#C9D3E0">${esc(a.derp || '—')}</td>
        <td class="${alerte ? 'down' : 'discret'}">${vu}</td>
        <td>${etat}</td></tr>`;
    }).join('');
    corps = `<section class="tableau defile" data-defile="appareils" style="flex:1" aria-label="Appareils du tailnet"><table>
      <thead><tr><th scope="col">Appareil</th><th scope="col">Adresse Tailscale</th><th scope="col">Relais DERP préféré</th><th scope="col">Dernière activité</th><th scope="col" style="width:150px">État</th></tr></thead>
      <tbody>${lignes}</tbody></table></section>`;
  }
  const resume = `<section class="resume" aria-label="Résumé">
    <div class="carte"><span class="lib">Cette tablette</span><strong>${esc(moi ? moi.nom : 'tablette')}</strong>
      <span class="sous">${ip ? `<span class="mono">${esc(ip)}</span> · sur le tailnet` : t.actif ? '<span class="down">hors du tailnet</span>' : 'Tailscale pas encore installé'}</span></div>
    <div class="carte"><span class="lib">Appareils</span><strong>${api.appareils ? `${enLigne.length} / ${appareils.length} en ligne` : '—'}</strong>
      <span class="sous${recents.length ? ' down' : ''}">${api.appareils ? (recents.length ? `${esc(recents.map((a) => a.nom).join(', '))} hors ligne` : 'aucune déconnexion récente') : 'clé API nécessaire'}</span></div>
    <div class="carte"><span class="lib">Relais DERP de la tablette</span><strong>${esc(moi && moi.derp ? moi.derp : '—')}</strong>
      <span class="sous">si pas de connexion directe</span></div>
    <div class="carte"><span class="lib">Exit node sur la tablette</span>${t.exit_node && t.exit_node.actif
      ? `<strong class="warn">Activé</strong><span class="sous warn">Internet passe par l'entreprise depuis ${duree(maintenant() - t.exit_node.depuis)}</span>`
      : `<strong>Non utilisé</strong><span class="sous">${sorties.length ? `Internet en direct · disponible : ${esc(sorties.join(', '))}` : 'Internet sort en direct'}</span>`}</div>
  </section>`;
  return `<div class="ecran">${rail('tailscale')}<main class="contenu">
    <header class="entete"><div class="titres"><div class="ligne-titre"><h1>Tailscale</h1>${etiquette}</div><p class="sous">${sous}</p></div>${horloge()}</header>
    ${api.erreur ? `<div class="alerte-api">${esc(api.erreur)}</div>` : ''}
    ${resume}${corps}
  </main></div>`;
}

// ------------------------------------------------------------------ Incidents
function pageIncidents() {
  const ouverts = S.etat.incidents_ouverts;
  const d = S.incidents;
  const resolus = d ? d.incidents.filter((i) => i.fin) : [];
  const f = S.filtre;
  const seg = (val, lib) => `<button type="button" class="appui" data-action="filtre" data-val="${val}" aria-pressed="${f === val}">${lib}</button>`;
  const cartes = ouverts.length ? ouverts.map((i) => {
    const href = i.service === 'tailscale' || i.service === 'exit-node' ? '#/tailscale' : `#/service/${encodeURIComponent(i.service)}`;
    return `<article class="incident ${i.niveau}"><div class="haut"><span class="icone">${iconeEtat(i.niveau, 26)}</span>
      <div class="textes"><strong>${esc(i.nom)} · ${i.niveau === 'down' ? 'hors ligne' : 'dégradé'}</strong>
      <span class="quand">Depuis ${heure(i.debut)} (${duree(maintenant() - i.debut)})</span>
      <span class="detail">${esc(i.message)}${i.acquitte ? ' · acquitté' : ''}</span></div></div>
      <div class="actions"><a href="${href}" class="bouton plein ${i.niveau} appui"><span>Voir</span>${svg(D.droite, 20, 2.2)}</a>
      ${i.acquitte ? '' : `<button type="button" class="bouton contour appui" data-action="acquitter" data-id="${i.id}">${svg(D.ok, 20, 2.2)}<span>Acquitter</span></button>`}</div></article>`;
  }).join('') : '<div class="vide">Aucun incident en cours.</div>';

  let liste = '';
  if (!d) liste = '<p class="discret">Chargement…</p>';
  else if (!resolus.length) liste = '<div class="vide">Aucun incident résolu sur les 30 derniers jours.</div>';
  else {
    const hier = new Date(); hier.setDate(hier.getDate() - 1);
    let jourCourant = '';
    liste = '<section class="liste" aria-label="Incidents résolus">';
    for (const i of resolus) {
      const dt = new Date(i.debut * 1000);
      const lib = memeJour(dt, new Date()) ? "aujourd'hui" : memeJour(dt, hier) ? `hier · ${jourLong(dt)}` : jourLong(dt);
      if (lib !== jourCourant) { liste += `<h3>${lib}</h3>`; jourCourant = lib; }
      liste += `<div class="resolu"><span class="marque ${i.niveau}"></span><strong>${esc(i.nom)}</strong>
        <span class="quoi">${esc(i.message)}</span><span class="plage">${heure(i.debut)} → ${heure(i.fin)}</span>
        <span class="duree">${duree(i.fin - i.debut)}</span></div>`;
    }
    liste += '</section>';
  }

  let bilan = '<section class="carte"><p class="discret">Chargement…</p></section>';
  if (d) {
    const b = d.bilan;
    const max = Math.max(1, ...b.par_service.map((x) => x.duree));
    const jauges = b.par_service.length ? b.par_service.map((x) => `<div class="jauge"><div class="lib"><span>${esc(x.nom)}</span>
      <span class="${x.en_cours ? 'warn' : 'discret'}">${duree(x.duree)}${x.en_cours ? ', en cours' : ''}</span></div>
      <div class="piste"><div class="${x.en_cours ? 'warn' : ''}" style="width:${(x.duree / max) * 100}%"></div></div></div>`).join('')
      : '<p class="discret" style="font-size:15px">Rien à signaler.</p>';
    bilan = `<section class="carte" aria-label="Bilan sur 30 jours" style="gap:14px"><h2>Bilan · 30 jours</h2>
      <div><div class="gros">${pct(b.dispo)}</div><span class="discret" style="font-size:14px">disponibilité moyenne des services</span></div>
      <div class="mini-grille"><div class="mini"><b>${b.nombre}</b><span>${b.nombre > 1 ? 'incidents' : 'incident'}</span></div>
      <div class="mini"><b>${b.duree ? duree(b.duree) : '0'}</b><span>de perturbation</span></div></div></section>
      <section class="carte defile" data-defile="parservice" style="flex:1;gap:14px" aria-label="Temps de perturbation par service"><h2>Par service</h2>${jauges}</section>`;
  }

  const gauche = [];
  if (f !== 'resolus') gauche.push(`<div class="titre-section"><h2>En cours</h2></div>${cartes}`);
  if (f !== 'encours') gauche.push(`<div class="titre-section" style="margin-top:8px"><h2>Résolus</h2></div>${liste}`);
  return `<div class="ecran">${rail('incidents')}<main class="contenu">
    <header class="entete"><div class="titres"><h1>Incidents</h1>
      <p class="sous">30 derniers jours · ${ouverts.length} en cours · ${d ? resolus.length : '…'} résolus</p></div>
      <div class="segments" role="group" aria-label="Filtrer">${seg('tous', 'Tous')}${seg('encours', 'En cours')}${seg('resolus', 'Résolus')}</div></header>
    <div class="deux-colonnes" style="grid-template-columns: minmax(0, 8fr) minmax(0, 4fr);">
      <div class="colonne defile" data-defile="incidents">${gauche.join('')}</div>
      <aside class="colonne">${bilan}</aside>
    </div>
  </main></div>`;
}

// ------------------------------------------------------------------ Mise a jour
const nomVersion = (v) => (!v || v === 'dev' ? 'dev' : `v${v}`);

// Notes de release (texte brut de GitHub) : lignes "- ..." en liste, le reste en paragraphes.
function notesRelease(texte) {
  const lignes = String(texte || '').split(/\r?\n/).map((l) => l.trim()).filter(Boolean);
  if (!lignes.length) return '<p class="discret">Pas de notes pour cette version.</p>';
  let html = '', liste = false;
  for (const l of lignes) {
    const puce = /^[-*] /.test(l);
    if (puce && !liste) { html += '<ul>'; liste = true; }
    if (!puce && liste) { html += '</ul>'; liste = false; }
    const t = esc(l.replace(/^[-*] /, '').replace(/^#+\s*/, ''));
    html += puce ? `<li>${t}</li>` : `<p>${t}</p>`;
  }
  return html + (liste ? '</ul>' : '');
}

function pageMaj() {
  const m = S.etat.maj;
  const d = m.disponible;
  const etiquettes = {
    a_jour: '<span class="etiquette up up"><span class="point"></span><span>À jour</span></span>',
    prete: '<span class="etiquette info">Nouvelle version disponible</span>',
    telechargement: '<span class="etiquette info">Téléchargement…</span>',
    erreur: '<span class="etiquette warn warn">Erreur</span>',
    desactive: '<span class="etiquette inconnu inconnu">Désactivée</span>',
    inconnu: '<span class="etiquette inconnu inconnu">Pas encore vérifiée</span>',
  };
  const etiquette = etiquettes[m.statut] || '';
  const sous = m.depot
    ? `Dépôt <span class="mono">${esc(m.depot)}</span> · ${m.en_cours ? 'vérification en cours…' : `vérifiée ${ilya(m.verifie)}`} · toutes les ${m.heures} h`
    : 'Ajoute une section [mise_a_jour] avec le dépôt GitHub dans config.toml';
  const bouton = m.depot
    ? `<button type="button" class="bouton appui" data-action="maj-verifier"${m.en_cours ? ' disabled' : ''}>${svg(D.relancer, 22, 2)}<span>${m.en_cours ? 'Vérification…' : 'Vérifier maintenant'}</span></button>` : '';

  const di = m.derniere_installation;
  const installee = di && di.version === m.actuelle
    ? `installée ${ilya(di.quand)} · avant : ${esc(nomVersion(di.depuis))}`
    : m.actuelle === 'dev' ? 'installée à la main, hors release' : 'version de la release';
  const gauche = `<section class="carte maj-actuelle"><span class="lib">Version installée</span>
      <strong class="gros-version">${esc(nomVersion(m.actuelle))}</strong><span class="discret">${installee}</span></section>
    <section class="carte maj-explication"><h2>Comment ça marche</h2>
      <p>À chaque push sur <span class="mono">main</span>, GitHub vérifie le code puis publie une release.</p>
      <p>La tablette la télécharge et contrôle son empreinte, mais n'installe rien sans ton accord.</p>
      <p>Ta configuration et l'historique sont conservés. Si la nouvelle version ne démarre pas, la tablette revient seule à la précédente.</p></section>`;

  let droite;
  if (d) {
    const prete = m.statut === 'prete';
    droite = `<section class="carte maj-dispo"><div class="maj-entete">
        <div><span class="lib">Nouvelle version</span><strong class="gros-version info">${esc(nomVersion(d.version))}</strong>
        <span class="discret">${d.publiee ? `publiée ${ilya(d.publiee)}` : ''}${prete ? ' · téléchargée et vérifiée (SHA-256)' : ' · téléchargement…'}</span></div>
        <button type="button" class="bouton plein info appui" data-action="maj-demander" data-version="${esc(d.version)}"${prete ? '' : ' disabled'}>${svg(D.maj, 22, 2.2)}<span>Mettre à jour</span></button></div>
      <div class="notes defile" data-defile="notes">${notesRelease(d.notes)}</div></section>`;
  } else if (m.statut === 'a_jour') {
    droite = `<section class="carte maj-vide">${svg(D.ok, 40, 2.4, HEX.up)}<strong>Tu as la dernière version.</strong>
      <span class="discret">La prochaine vérification se fera automatiquement.</span></section>`;
  } else {
    droite = `<section class="carte maj-vide">${svg(D.maj, 40, 2, '#93A1B5')}<strong>${m.depot ? 'Aucune nouvelle version pour le moment.' : 'Mises à jour désactivées.'}</strong></section>`;
  }
  return `<div class="ecran">${rail('maj')}<main class="contenu">
    <header class="entete"><div class="titres"><div class="ligne-titre"><h1>Mise à jour</h1>${etiquette}</div><p class="sous">${sous}</p></div>${bouton}${horloge()}</header>
    ${m.erreur ? `<div class="alerte-api">${esc(m.erreur)}</div>` : ''}
    <div class="deux-colonnes" style="grid-template-columns: minmax(0, 5fr) minmax(0, 7fr);">
      <div class="colonne">${gauche}</div><div class="colonne">${droite}</div>
    </div>
  </main></div>`;
}

function fenetreMaj() {
  const m = S.etat && S.etat.maj;
  const enCours = S.majEnvoyee || (m && (m.statut === 'installation' || m.statut === 'redemarrage'));
  if (enCours) {
    const v = S.majEnvoyee || (m.disponible && m.disponible.version);
    return `<div class="installation"><span class="roue"></span><strong>Installation de ${esc(nomVersion(v))}…</strong>
      <span>L'appli redémarre, l'écran va se recharger tout seul.</span></div>`;
  }
  if (!S.confirmerMaj) return '';
  return `<div class="modale-fond" data-action="maj-annuler"><div class="modale" role="dialog" aria-modal="true" data-action="rien">
    <h2>Installer ${esc(nomVersion(S.confirmerMaj))} ?</h2>
    <p>L'appli va redémarrer, ce qui prend une vingtaine de secondes. Ta configuration et l'historique sont conservés.</p>
    <p>Si la nouvelle version ne démarre pas, la tablette revient automatiquement à ${esc(nomVersion(m.actuelle))}.</p>
    <div class="actions"><button type="button" class="bouton appui" data-action="maj-annuler">Annuler</button>
      <button type="button" class="bouton plein info appui" data-action="maj-confirmer">${svg(D.maj, 22, 2.2)}<span>Oui, mettre à jour</span></button></div>
  </div></div>`;
}

// ------------------------------------------------------------------ Veille
function pageVeille() {
  const e = S.etat;
  const pbs = problemes();
  const d = new Date();
  let niveau = 'up', titre = 'Tout est opérationnel', sous;
  const tous = e.services.flatMap((s) => s.nodes);
  const t = e.tailscale;
  if (pbs.length === 1) {
    niveau = pbs[0].niveau; titre = pbs[0].court; sous = `${pbs[0].detail} · depuis ${duree(maintenant() - pbs[0].depuis)}`;
  } else if (pbs.length > 1) {
    niveau = pire(pbs); titre = `${pbs.length} problèmes en cours`; sous = pbs.map((p) => p.court).join(' · ');
  } else {
    sous = `${pluriel(e.services.length, 'service')} · ${pluriel(tous.length, 'node')}${t.tablette.ip ? ' · Tailscale connecté' : ''}`;
  }
  const points = e.services.map((s) => `<span class="${s.etat === 'up' ? '' : s.etat}"><i class="${s.etat === 'up' ? '' : s.etat}"></i>${esc(s.nom)}</span>`).join('');
  return `<a href="#/" class="veille" aria-label="Ouvrir le tableau de bord">
    ${niveau !== 'up' ? `<span class="barre ${niveau}"></span>` : ''}
    <div class="centre">
      <div><div class="heure">${d.toLocaleTimeString('fr-FR', { hour: '2-digit', minute: '2-digit' })}</div><div class="date">${jourLong(d)}</div></div>
      <div class="statut"><div>${iconeEtat(niveau, 36)}<span style="font-size:34px;color:${niveau === 'up' ? '#DCE3EC' : HEX[niveau]}">${esc(titre)}</span></div><span>${esc(sous)}</span></div>
    </div>
    <div class="pied"><div class="points">${points}</div><span class="indication">Touche l'écran pour ouvrir le tableau de bord</span></div>
  </a>`;
}

// ------------------------------------------------------------------ routage et rendu
function route() {
  const h = location.hash.replace(/^#\/?/, '');
  if (h.startsWith('service/')) return { page: 'service', id: decodeURIComponent(h.slice(8)) };
  if (h === 'tailscale' || h === 'incidents' || h === 'veille' || h === 'maj') return { page: h };
  return { page: 'accueil' };
}

function rendre() {
  if (S.doigt) { S.renduEnAttente = true; return; }
  S.renduEnAttente = false;
  if (!S.etat) {
    app.innerHTML = `<div class="demarrage">${S.erreur ? 'Serveur de monitoring injoignable, nouvelle tentative…' : 'Démarrage du monitoring…'}</div>`;
    return;
  }
  const defile = {};
  app.querySelectorAll('[data-defile]').forEach((el) => { defile[el.dataset.defile] = el.scrollTop; });
  const r = route();
  let html;
  if (r.page === 'service') html = pageService(r.id);
  else if (r.page === 'tailscale') html = pageTailscale();
  else if (r.page === 'incidents') html = pageIncidents();
  else if (r.page === 'veille') html = pageVeille();
  else if (r.page === 'maj') html = pageMaj();
  else html = pageAccueil();
  html += fenetreMaj();
  if (S.erreur && !S.majEnvoyee) html += '<div class="hors-ligne">Serveur de monitoring injoignable : données figées</div>';
  if (S.toast) html += `<div class="toast">${esc(S.toast)}</div>`;
  app.innerHTML = html;
  app.querySelectorAll('[data-defile]').forEach((el) => { if (defile[el.dataset.defile]) el.scrollTop = defile[el.dataset.defile]; });
}

async function lireJSON(url, options) {
  const rep = await fetch(url, Object.assign({ cache: 'no-store' }, options));
  if (!rep.ok) throw new Error(`HTTP ${rep.status}`);
  return rep.json();
}

async function chargerPage(force) {
  const r = route();
  try {
    if (r.page === 'service' && (force || !S.service || S.service.id !== r.id || Date.now() - S.serviceAt > 30000)) {
      S.service = await lireJSON(`/api/service/${encodeURIComponent(r.id)}`);
      S.serviceAt = Date.now();
    }
    if (r.page === 'incidents' && (force || !S.incidents || Date.now() - S.incidentsAt > 30000)) {
      S.incidents = await lireJSON('/api/incidents');
      S.incidentsAt = Date.now();
    }
  } catch (e) { /* on reessaie au prochain cycle */ }
}

function verifierReveil() {
  const ids = new Set(S.etat.incidents_ouverts.filter((i) => !i.acquitte).map((i) => i.id));
  if (S.ouvertsConnus) {
    const nouveau = [...ids].some((id) => !S.ouvertsConnus.has(id));
    if (nouveau) {
      S.derniereAction = Date.now();
      if (route().page === 'veille') location.hash = '#/';
      S.incidentsAt = 0;
    }
  }
  S.ouvertsConnus = ids;
}

async function rafraichir() {
  try {
    S.etat = await lireJSON('/api/etat');
    S.erreur = false;
    verifierReveil();
    const m = S.etat.maj, di = m && m.derniere_installation;
    if (S.majEnvoyee && m && m.statut === 'prete' && m.erreur) S.majEnvoyee = null;   // installation refusee
    if (!S.toastMajVu && di && di.version === m.actuelle && maintenant() - di.quand < 180) {
      S.toastMajVu = true;
      setTimeout(() => afficherToast(`Mise à jour installée : ${nomVersion(m.actuelle)}`), 500);
    }
  } catch (e) {
    S.erreur = true;
  }
  await chargerPage(false);
  rendre();
}

function afficherToast(texte) {
  S.toast = texte;
  rendre();
  setTimeout(() => { S.toast = null; rendre(); }, 2500);
}

document.addEventListener('click', async (ev) => {
  const el = ev.target.closest('[data-action]');
  if (!el) return;
  ev.preventDefault();
  const action = el.dataset.action;
  try {
    if (action === 'rien') return;
    if (action === 'filtre') { S.filtre = el.dataset.val; rendre(); return; }
    if (action === 'maj-demander') { S.confirmerMaj = el.dataset.version; rendre(); return; }
    if (action === 'maj-annuler') { S.confirmerMaj = null; rendre(); return; }
    if (action === 'maj-verifier') {
      await lireJSON('/api/maj/verifier', { method: 'POST' });
      afficherToast('Vérification lancée');
      setTimeout(rafraichir, 4000);
    }
    if (action === 'maj-confirmer') {
      const version = S.confirmerMaj;
      S.confirmerMaj = null;
      const rep = await fetch('/api/maj/installer', {
        method: 'POST', cache: 'no-store', headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ version }),
      });
      if (!rep.ok) {
        const d = await rep.json().catch(() => ({}));
        afficherToast(d.erreur || 'Installation refusée');
        return;
      }
      S.majEnvoyee = version;
      rendre();
    }
    if (action === 'relancer') {
      await lireJSON(`/api/controler/${encodeURIComponent(el.dataset.id)}`, { method: 'POST' });
      afficherToast('Contrôles relancés');
      setTimeout(async () => { S.serviceAt = 0; await rafraichir(); }, 3000);
    }
    if (action === 'acquitter') {
      await lireJSON(`/api/incidents/${el.dataset.id}/acquitter`, { method: 'POST' });
      afficherToast('Incident acquitté');
      await rafraichir();
    }
  } catch (e) {
    afficherToast('Action impossible : serveur injoignable');
  }
});

document.addEventListener('pointerdown', () => { S.doigt = true; S.derniereAction = Date.now(); }, true);
const relacher = () => { S.doigt = false; if (S.renduEnAttente) setTimeout(rendre, 50); };
document.addEventListener('pointerup', relacher, true);
document.addEventListener('pointercancel', relacher, true);
document.addEventListener('contextmenu', (ev) => ev.preventDefault());

window.addEventListener('hashchange', async () => {
  S.derniereAction = Date.now();
  S.doigt = false;
  rendre();
  await chargerPage(true);
  rendre();
});

setInterval(() => {
  if (!S.etat) return;
  const minutes = S.etat.veille_minutes;
  if (minutes > 0 && route().page !== 'veille' && !S.majEnvoyee && Date.now() - S.derniereAction > minutes * 60000) {
    S.confirmerMaj = null;
    location.hash = '#/veille';
  }
}, 10000);

rafraichir();
setInterval(rafraichir, 5000);
