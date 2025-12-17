module.exports.defineConfig = (config) => config;
module.exports.dir = (children) => ({ type: "dir", children: children || {} });
module.exports.file = (pattern) => ({ type: "file", pattern });
module.exports.opt = (node) => ({ ...node, optional: true });
module.exports.param = (opts, child) => ({ type: "param", ...opts, child });
module.exports.many = (optsOrChild, child) => {
  if (child) return { type: "many", ...optsOrChild, child };
  return { type: "many", child: optsOrChild };
};
module.exports.recursive = (optsOrChild, child) => {
  if (child) return { type: "recursive", ...optsOrChild, child };
  return { type: "recursive", maxDepth: 10, child: optsOrChild };
};
module.exports.either = (...variants) => ({ type: "either", variants });

const presets = require('./presets/index.js');
module.exports.nextjsAppRouter = presets.nextjsAppRouter;
module.exports.nextjsDefaultIgnore = presets.nextjsDefaultIgnore;
module.exports.nextjsDefaultIgnorePaths = presets.nextjsDefaultIgnorePaths;
