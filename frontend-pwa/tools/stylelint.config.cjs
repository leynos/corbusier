module.exports = {
  plugins: ["stylelint-declaration-strict-value"],
  rules: {
    "color-no-hex": true,
    "color-named": "never",
    "scale-unlimited/declaration-strict-value": [
      [
        "/^color$/",
        "^background(-color)?$",
        "^border(-.*)?-color$",
        "^outline-color$",
        "fill",
        "stroke",
      ],
      { ignoreValues: ["transparent", "currentColor", "/^var\\(--/"] },
    ],
    "declaration-no-important": true,
    "declaration-property-unit-disallowed-list": {
      "/^(margin|padding|gap|inset|top|right|bottom|left)$/": ["px"],
    },
  },
};
