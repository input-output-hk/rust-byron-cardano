export const apply = (fn, ...partials) => (...args) => fn(...partials, ...args);
