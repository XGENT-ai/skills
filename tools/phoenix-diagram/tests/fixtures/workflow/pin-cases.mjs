// Inputs adapted from the frozen workflow hard-contract tests. This module
// supplies source specimens only; generate-pins.mjs obtains every expectation.
const source = 'local/phoenix-diagram/reference/test/workflow-compiler-hard-contract.test.mjs';
const workflow = (lanes, nodes, edges) => ({
  schema_version: 2, diagram_type: 'workflow',
  meta: { title: 'Workflow compiler hard-contract fixture', output: 'workflow-hard-contract.html', legend: { mode: 'hidden' } },
  lanes: lanes.map(id => ({ id, label: id })),
  nodes: nodes.map(([id, lane, col]) => ({ id, lane, col, type: 'backend', label: id.toUpperCase() })),
  edges,
});
const twoLanes = edges => workflow(['top', 'bottom'], [['a','top',0],['b','top',4],['c','bottom',0],['d','bottom',4]], edges);
const oneLane = edges => workflow(['main'], [['a','main',0],['b','main',2]], edges);
const crossing = (horizontal = {}, vertical = {}) => workflow(['top','middle','bottom'], [['left','middle',0],['right','middle',4],['above','top',2],['below','bottom',2]], [
  { id: 'a-auto', from: 'left', to: 'right', ...horizontal },
  { id: 'z-pinned', from: 'above', to: 'below', via: [[334,243]], ...vertical },
]);
const documents = [];
const add = (name, line, document, profiles = ['standard','showcase']) => {
  for (const quality of profiles) documents.push({name:`${name}/${quality}`, documentSource:`${source}:${line}`, document:{...document,meta:{...document.meta,quality_profile:quality}}, quality});
};
add('absolute-label-route',1538,twoLanes([
  {id:'labelpin',from:'a',to:'b',label:'PIN',labelAt:[334,243]},
  {id:'routepin',from:'c',to:'d',via:[[334,243]]},
]));
add('fractional-label-route',1538,twoLanes([
  {id:'labelpin',from:'a',to:'b',label:'PIN',labelAt:[334.123456789,243.123456789]},
  {id:'routepin',from:'c',to:'d',via:[[334,243]]},
]));
add('route-first-label-route',1566,twoLanes([
  {id:'a-route',from:'c',to:'d',via:[[334,243]]},
  {id:'z-label',from:'a',to:'b',label:'PIN',labelAt:[334,243]},
]));
add('label-first-label-route',1600,twoLanes([
  {id:'z-route',from:'c',to:'d',via:[[334,243]]},
  {id:'a-label',from:'a',to:'b',label:'PIN',labelAt:[334,243]},
]));
add('label-pin-prior-automatic-route',1644,twoLanes([
  {id:'a-route',from:'c',to:'d'},
  {id:'z-label',from:'a',to:'b',label:'PIN',labelAt:[334,243]},
]));
add('automatic-label-later-pinned-route',1692,workflow(['top','mid','bottom'], [['a','mid',0],['b','mid',4],['c','top',2],['d','bottom',2]], [
  {id:'a-label',from:'a',to:'b',label:'AUTO'},
  {id:'z-route',from:'c',to:'d',via:[[334,243]]},
]));
add('label-pin-prior-automatic-label',1748,oneLane([
  {id:'a-auto',from:'a',to:'b',label:'AUTO'},
  {id:'z-pin',from:'a',to:'b',label:'PIN',labelAt:[214,119]},
]));
add('causal-label-label',1781,twoLanes([
  {id:'a-label',from:'a',to:'b',label:'FIRST',labelAt:[334,119]},
  {id:'z-label',from:'c',to:'d',label:'SECOND',labelAt:[334,119]},
]));
add('two-pinned-labels',1816,oneLane([
  {id:'one',from:'a',to:'b',label:'ONE',via:[[214,119]],labelAt:[214,80]},
  {id:'two',from:'a',to:'b',label:'TWO',via:[[214,119]],labelAt:[214,80]},
]));
add('forward-collinear-crossing',1875,crossing());
add('two-forward-collinear-crossing',1907,crossing({id:'a-pinned',via:[[334,243]]}));
add('side-only-crossing',1927,workflow(['l0','l1','l2'], [['n00','l0',0],['n01','l0',1],['n02','l0',2],['n11','l1',1]], [
  {id:'a-auto',from:'n00',to:'n01'},
  {id:'z-side',from:'n02',to:'n11',fromSide:'top',toSide:'left'},
]));
add('side-only-corridor',1964,workflow(['l0','l1','l2'], [['n00','l0',0],['n02','l0',2],['n01','l0',1],['n03','l0',3]], [
  {id:'a-auto',from:'n00',to:'n02'},
  {id:'z-side',from:'n01',to:'n03',fromSide:'bottom',toSide:'top'},
]));
add('nested-outside-right-corridor',2179,workflow(['l0','l1','l2','l3'], [['outer-from','l0',0],['inner-from','l1',0],['inner-to','l2',2],['outer-to','l3',2]], [
  {id:'outer',from:'outer-from',to:'outer-to',route:'outside-right',channelX:800,fromSide:'right',toSide:'right'},
  {id:'inner',from:'inner-from',to:'inner-to',route:'outside-right',channelX:800,fromSide:'right',toSide:'right'},
]));
add('earlier-label-pin-unrelated-self-failure',2233,oneLane([
  {id:'a-pinned',from:'a',to:'b',label:'Pinned',labelAt:[94,30]},
  {id:'z-self',from:'b',to:'b',label:'Retry'},
]));
for(const quality of ['standard','showcase']) {
  const document=oneLane([
    {id:'a-pinned',from:'a',to:'b',label:'Pinned',labelAt:[420,112]},
    {id:'z-self',from:'b',to:'b',label:'Retry'},
  ]);
  documents.push({name:`earlier-solver-before-later-pin-candidate/${quality}`,documentSource:`${source}:2233`,contracts:['phoenix-v1'],document:{...document,meta:{...document.meta,quality_profile:quality}},quality});
  const auto=structuredClone(document);Object.assign(auto.edges[1],{fromSide:'auto',toSide:'auto'});
  documents.push({name:`earlier-solver-auto-sides-before-later-pin/${quality}`,documentSource:`${source}:2233`,contracts:['phoenix-v1'],document:{...auto,meta:{...auto.meta,quality_profile:quality}},quality});
}
// Same crossing geometry with redundant authoritative controls exercises
// removal cardinality: deleting one field must not be advertised if it fails.
add('crossing-redundant-controls',88,crossing(
  {id:'a-pinned',via:[[334,243]],channelY:243,fromSide:'right',toSide:'left'},
  {channelX:334,fromSide:'bottom',toSide:'top'},
));
for (const [kind, collisionOffset, clearOffset] of [['label-label',124,112],['route-label',138,148],['label-route',-110,-122]]) {
  for(const [state,offset] of [['conflict',collisionOffset],['boundary',clearOffset]]) {
    const document=workflow(['top','bottom'],[['a','top',0],['b','top',2],['c','bottom',0],['d','bottom',2]],[
      {...(kind==='label-route'?{}:{id:'a-placed',label:'Placed',labelDy:offset}),from:'a',to:'b',route:'straight'},
      {id:'z-candidate',from:'c',to:'d',route:'straight',label:'Candidate',...(kind==='label-route'?{labelDy:offset}:{})},
    ]);
    document.edges.reverse();
    for(const quality of ['standard','showcase'])documents.push({name:`placed-${kind}-${state}/${quality}`,documentSource:'local/phoenix-diagram/reference/test/workflow-placed-label-evidence.test.mjs:6',document:{...document,meta:{...document.meta,quality_profile:quality}},quality});
  }
}
const outsideRightLabel=()=>({
  schema_version:2,diagram_type:'workflow',
  meta:{title:'Workflow compiler hard-contract fixture',output:'workflow-hard-contract.html',legend:{mode:'hidden'}},
  lanes:[{id:'undo',label:'Undo'},{id:'return',label:'Follow-up'}],
  nodes:[
    {id:'receipt',lane:'undo',col:5,type:'database',label:'Receipt'},
    {id:'rescan',lane:'return',col:5,type:'backend',label:'Rescan'},
  ],
  edges:[{id:'follow-up',from:'receipt',to:'rescan',label:'成功后按需另行 Scan',role:'branch',variant:'dashed',route:'outside-right',fromSide:'right',toSide:'right'}],
});
add('preset-label-local-segment',849,outsideRightLabel());
for(const field of ['labelSegment','labelDx','labelDy']) {
  const document=outsideRightLabel();document.edges[0][field]=0;
  add(`preset-label-authored-zero-${field}`,935,document,['standard']);
}
const incomplete=outsideRightLabel();
Object.assign(incomplete.nodes[0],{width:132,sublabel:'chain / debate / synthesis 调度'});
for(const col of [0,1,2,3,4])incomplete.nodes.push({id:`wide-${col}`,lane:'undo',col,type:'backend',label:`Step ${col}`,width:200});
add('preset-label-incomplete-workflow-repair',914,incomplete,['showcase']);
const blocked=workflow(['top','bottom'],[],[{id:'retry',from:'source',to:'target',route:'outside-right',fromSide:'right',toSide:'right',role:'return',label:'Try payment again',variant:'dashed'}]);
blocked.nodes=[
  {id:'target',lane:'top',col:4,type:'backend',label:'Target'},
  {id:'obstacle',lane:'top',col:5,type:'backend',label:'Obstacle'},
  {id:'source',lane:'bottom',col:4,width:104,type:'backend',label:'Source'},
];
add('preset-label-route-node-priority',815,blocked);
export default documents;
