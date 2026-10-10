import fs from 'node:fs';
import path from 'node:path';

const reservation = 'oracle collision reservation\n';

export function reserveCritiqueCollisions(ws, timeoutMs = 60_000, now = Date.now()) {
  const started = Date.now();
  const directory = path.join(ws, '.phoenix-ui/critique');
  fs.mkdirSync(directory, { recursive: true });
  const created = [];
  // Preparation and the verb each get a timeout window, plus the partial second.
  const start = Math.floor(now / 1000) * 1000;
  for (let second = 0; second <= Math.ceil(timeoutMs / 1000) * 2 + 1; second++) {
    const stamp = new Date(start + second * 1000).toISOString().replace(/:/g, '-').replace(/\.\d{3}Z$/, 'Z');
    const file = path.join(directory, `${stamp}__src-app-tsx.md`);
    try {
      fs.writeFileSync(file, reservation, { flag: 'wx' });
      created.push(file);
    } catch (error) {
      if (error.code !== 'EEXIST') throw error;
    }
  }
  if (Date.now() - started > timeoutMs) {
    removeCritiqueCollisions(created);
    throw new Error('Critique collision preparation exceeded its timeout');
  }
  return created;
}

export function removeCritiqueCollisions(created) {
  for (const file of created) {
    if (!fs.lstatSync(file).isFile() || fs.readFileSync(file, 'utf8') !== reservation) {
      throw new Error('Critique writer overwrote a collision reservation');
    }
  }
  for (const file of created) fs.unlinkSync(file);
}
