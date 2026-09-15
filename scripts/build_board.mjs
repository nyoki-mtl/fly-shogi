import { build } from 'esbuild';
await build({entryPoints:['ui/shogilens/entry.tsx'],bundle:true,outfile:'web/shogilens-board.js',format:'iife',jsx:'automatic',minify:true,define:{'process.env.NODE_ENV':'"production"'},legalComments:'linked'});
