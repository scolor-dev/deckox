import eslint from "@eslint/js";
import eslintPluginVue from "eslint-plugin-vue";
import globals from "globals";
import typescriptEslint from "typescript-eslint";

export default typescriptEslint.config(
  {
    ignores: ["**/*.d.ts", "dist", "node_modules"],
  },
  {
    files: ["**/*.{ts,vue}"],
    extends: [
      eslint.configs.recommended,
      ...typescriptEslint.configs.strictTypeChecked,
      ...typescriptEslint.configs.stylisticTypeChecked,
      ...eslintPluginVue.configs["flat/recommended-error"],
    ],
    languageOptions: {
      ecmaVersion: "latest",
      sourceType: "module",
      globals: globals.browser,
      parserOptions: {
        extraFileExtensions: [".vue"],
        parser: typescriptEslint.parser,
        projectService: true,
        tsconfigRootDir: import.meta.dirname,
      },
    },
    rules: {
      "@typescript-eslint/consistent-type-imports": [
        "error",
        { fixStyle: "inline-type-imports" },
      ],
      "@typescript-eslint/no-floating-promises": "error",
      "@typescript-eslint/no-unnecessary-condition": "error",
      "@typescript-eslint/prefer-nullish-coalescing": "error",
      "vue/block-lang": ["error", { script: { lang: "ts" } }],
      "vue/component-api-style": ["error", ["script-setup"]],
    },
  },
  {
    files: ["vite.config.ts"],
    languageOptions: {
      globals: globals.node,
    },
  },
);
