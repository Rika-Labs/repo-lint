module.exports.defineConfig = (config) => config;

const directory = (children, options) => ({
  type: "dir",
  children: children || {},
  ...(options?.strict && { strict: true }),
  ...(options?.maxDepth !== undefined && { maxDepth: options.maxDepth }),
});
module.exports.directory = directory;
module.exports.dir = directory;

const file = (patternOrOptions) => {
  if (typeof patternOrOptions === "string" || patternOrOptions === undefined) {
    return { type: "file", pattern: patternOrOptions };
  }
  return {
    type: "file",
    pattern: patternOrOptions.pattern,
    ...(patternOrOptions.case && { case: patternOrOptions.case }),
  };
};
module.exports.file = file;

const optional = (node) => ({ ...node, optional: true });
module.exports.optional = optional;
module.exports.opt = optional;

const required = (node) => ({ ...node, required: true });
module.exports.required = required;

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
