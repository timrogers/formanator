#!/usr/bin/env npx ts-node --esm

import * as commander from "commander";

import login from "./commands/login.js";
import benefits from "./commands/benefits.js";
import categories from "./commands/categories.js";
import claim from "./commands/claim.js";

const program = new commander.Command();

program
  .addCommand(login)
  .addCommand(benefits)
  .addCommand(categories)
  .addCommand(claim);

program.parse();
