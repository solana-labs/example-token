module.exports = {
  // eslint-disable-line import/no-commonjs
  "parser": "babel-eslint",
  "env": {
  },
  "plugins": [            
      "flowtype"        
  ],
  "extends": [
      "eslint:recommended",
      "plugin:flowtype/recommended",
      'plugin:jest/recommended',
      '../.eslintrc.js'
  ],
  "rules": {            
  }
};
