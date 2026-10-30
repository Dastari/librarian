import { readFile, readdir, mkdir, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
const root=fileURLToPath(new URL('../../../',import.meta.url)).replace(/\/$/,'');
const { buildSchema, parse, Kind, concatAST, validate } = await import(root+'/web/node_modules/graphql/index.mjs');
const schema=buildSchema(await readFile(root+'/web/schema.graphql','utf8'));
const files=[];async function walk(dir){for(const entry of (await readdir(dir,{withFileTypes:true})).sort((a,b)=>a.name.localeCompare(b.name))){const path=dir+'/'+entry.name;if(entry.isDirectory())await walk(path);else if(path.endsWith('.graphql'))files.push(path);}}
await walk(root+'/web/src/graphql/entities');await walk(root+'/web/src/graphql/documents');
const documents=[],operations=[],generated={query:new Set(),mutation:new Set(),subscription:new Set()};
for(const path of files){const ast=parse(await readFile(path,'utf8'));documents.push(ast);for(const d of ast.definitions){if(d.kind!==Kind.OPERATION_DEFINITION)continue;const fields=d.selectionSet.selections.filter(s=>s.kind===Kind.FIELD).map(s=>s.name.value);operations.push({file:path.slice(root.length+1),name:d.name?.value,operation:d.operation,fields});if(path.includes('/entities/'))fields.forEach(f=>generated[d.operation].add(f));}}
const roots={};for(const [kind,type] of [['query',schema.getQueryType()],['mutation',schema.getMutationType()],['subscription',schema.getSubscriptionType()]])roots[kind]=Object.values(type.getFields()).map(f=>({name:f.name,type:String(f.type),args:f.args.map(a=>({name:a.name,type:String(a.type),...(a.defaultValue===undefined?{}:{default:a.defaultValue})})),entityDocument:generated[kind].has(f.name),operations:operations.filter(o=>o.operation===kind&&o.fields.includes(f.name)).map(o=>o.name)}));
const errors=validate(schema,concatAST(documents)).map(e=>e.message);
const output={date:'2026-10-24',authority:'Committed web/schema.graphql snapshot; NOT a runtime export of newly pinned backend',dependencyPins:{graphqlOrm:'0.57.0@a90b5bd41f952b02416f23999c729b9ceffcbb8d',agqlAuth:'0.22.1@86fabec09eea1eec0f675bc150e90e3809734567'},counts:{documentFiles:files.length,operationDefinitions:operations.length,roots:Object.fromEntries(Object.entries(roots).map(([k,v])=>[k,{total:v.length,representedByEntityDocuments:v.filter(x=>x.entityDocument).length,other:v.filter(x=>!x.entityDocument).length}])),operationDefinitionsByType:Object.fromEntries(['query','mutation','subscription'].map(k=>[k,operations.filter(o=>o.operation===k).length]))},validation:{errors,passed:errors.length===0},roots,operations};
await mkdir(root+'/docs/audits/2026-10-24',{recursive:true});await writeFile(root+'/docs/audits/2026-10-24/schema-inventory.json',JSON.stringify(output,null,2)+'\n');console.log(JSON.stringify(output.counts,null,2));console.log('Document validation errors',errors.length);

if(errors.length)process.exitCode=1;
