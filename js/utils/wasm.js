export const applyModule = (loadModule, target) => (
  (...args) => loadModule().then((module) => target(module, ...args))
);
