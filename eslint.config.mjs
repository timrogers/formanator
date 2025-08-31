import prettier from "eslint-plugin-prettier";
import typescriptEslint from "@typescript-eslint/eslint-plugin";
import globals from "globals";
import tsParser from "@typescript-eslint/parser";
import path from "node:path";
import { fileURLToPath } from "node:url";
import js from "@eslint/js";
import { FlatCompat } from "@eslint/eslintrc";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const compat = new FlatCompat({
    baseDirectory: __dirname,
    recommendedConfig: js.configs.recommended,
    allConfig: js.configs.all
});

export default [
    {
        ignores: ["dist/*", "script/*", "eslint.config.mjs", "coverage/**/*", "node_modules/**/*"],
    },
    ...compat.extends("prettier", "eslint:recommended", "plugin:@typescript-eslint/recommended"),
    {
        files: ["**/*.ts"],
        plugins: {
            prettier,
            "@typescript-eslint": typescriptEslint,
        },

        languageOptions: {
            globals: {
                ...globals.browser,
                ...globals.node,
            },

            parser: tsParser,
            ecmaVersion: "latest",
            sourceType: "module",

            parserOptions: {
                project: "./tsconfig.json",
            },
        },

        rules: {
            "@typescript-eslint/strict-boolean-expressions": "off",
            "prettier/prettier": "error",
        },
    },
    {
        files: ["**/*.js"],
        plugins: {
            prettier,
        },

        languageOptions: {
            globals: {
                ...globals.browser,
                ...globals.node,
                ...globals.jest,
            },

            ecmaVersion: "latest",
            sourceType: "module",
        },

        rules: {
            "prettier/prettier": "error",
        },
    }
];