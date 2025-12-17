module.exports.defineConfig = (config) => config;
module.exports.dir = (children) => ({ type: "dir", children: children || {} });
module.exports.file = (pattern) => ({ type: "file", pattern });
module.exports.opt = (node) => ({ ...node, optional: true });
module.exports.param = (opts, child) => ({ type: "param", ...opts, child });
module.exports.many = (optsOrChild, child) => {
  if (child) return { type: "many", ...optsOrChild, child };
  return { type: "many", child: optsOrChild };
};
