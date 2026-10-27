// Validate the exact JavaScript embedded into gxoled.exe, using Node's parser.
const fs = require('node:fs');
const vm = require('node:vm');
const path = require('node:path');
const html = fs.readFileSync(path.join(__dirname, 'editor.html'), 'utf8');
const match = html.match(/<script>([\s\S]*?)<\/script>/);
if (!match) throw new Error('Embedded GIF Studio script is missing');
new vm.Script(match[1], { filename: 'studio/editor.html' });
const buttons = ['import','addText','openProject','saveProject','send','exportGif','play','scrub'];
for (const id of buttons) {
  if (!html.includes(`id="${id}"`)) throw new Error(`Studio control missing: ${id}`);
}
console.log('Studio JavaScript parsed; core editing controls are present.');
