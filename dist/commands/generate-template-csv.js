import * as commander from 'commander';
import { existsSync, writeFileSync } from 'fs';
import chalk from 'chalk';
import { actionRunner } from '../utils.js';
import VERSION from '../version.js';
const command = new commander.Command();
const TEMPLATE_CSV = `benefit,category,merchant,amount,description,purchaseDate,receiptPath\n`;
command
    .name('generate-template-csv')
    .version(VERSION)
    .description('Generate a template CSV for submitting multiple claims at the same time')
    .option('--output-path <output_path>', 'The path to write the CSV to', 'claims.csv')
    .action(actionRunner(async (opts) => {
    if (existsSync(opts.outputPath)) {
        throw new Error(`File '${opts.outputPath}' already exists. Please delete it first, or set a different \`--output-path\` option.`);
    }
    writeFileSync(opts.outputPath, TEMPLATE_CSV);
    console.log(chalk.green(`Wrote template CSV to ${opts.outputPath}`));
}));
export default command;
