const path = require('path');

module.exports = {
  devtool: 'cheap-source-map',
  entry: {
    index: './js/index.js',
  },
  output: {
    filename: './dist/index.js',
    library: 'CardanoCrypto',
    libraryTarget: 'umd',
  },
  cache: true,
  module: {
    rules: [
      {
        test: /\.js$/,
        use: 'babel-loader',
      },
      {
        test: /\.wasm$/,
        use: 'wasm-loader'
      }
    ]
  },
};
