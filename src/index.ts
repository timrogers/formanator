#!/usr/bin/env node

import * as commander from 'commander';

import login from './commands/login.js';
import benefits from './commands/benefits.js';
import categories from './commands/categories.js';
import listClaims from './commands/list-claims.js';
import submitClaim from './commands/submit-claim.js';
import generateTemplateCsv from './commands/generate-template-csv.js';
import submitClaimsFromCsv from './commands/submit-claims-from-csv.js';
import submitClaimsFromDirectory from './commands/submit-claims-from-directory.js';
import validateCsv from './commands/validate-csv.js';
import mcp from './commands/mcp.js';
import VERSION from './version.js';

const program = new commander.Command();

program
  .version(VERSION)
  .addCommand(login)
  .addCommand(benefits)
  .addCommand(categories)
  .addCommand(listClaims)
  .addCommand(submitClaim)
  .addCommand(generateTemplateCsv)
  .addCommand(submitClaimsFromCsv)
  .addCommand(submitClaimsFromDirectory)
  .addCommand(validateCsv)
  .addCommand(mcp);

program.parse();
