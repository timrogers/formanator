import * as commander from 'commander';
import { existsSync } from 'fs';
import chalk from 'chalk';

import { actionRunner, serializeError } from '../utils.js';
import { getBenefitsWithCategories } from '../forma.js';
import { claimParamsToCreateClaimOptions, readClaimsFromCsv } from '../claims.js';
import VERSION from '../version.js';
import { getAccessToken } from '../config.js';

const command = new commander.Command();

interface Arguments {
  accessToken?: string;
  inputPath: string;
}

command
  .name('validate-csv')
  .version(VERSION)
  .description(
    'Validate a completed CSV before submitting it with `npx formanator submit-claims-from-csv`',
  )
  .requiredOption('--input-path <input_path>', 'The path to the CSV to read claims from')
  .action(
    actionRunner(async (opts: Arguments) => {
      const accessToken = opts.accessToken ?? getAccessToken();

      if (!accessToken) {
        throw new Error(
          "You aren't logged in to Forma. Please run `npx formanator login` first.",
        );
      }

      if (!existsSync(opts.inputPath)) {
        throw new Error(`File '${opts.inputPath}' doesn't exist.`);
      }

      const claims = await readClaimsFromCsv(opts.inputPath);

      if (!claims.length) {
        throw new Error(
          "Your CSV doesn't seem to contain any claims. Have you filled out the template?",
        );
      }

      const benefitsWithCategories = await getBenefitsWithCategories(accessToken);

      for (const [index, claim] of claims.entries()) {
        const rowNumber = index + 2;
        console.log(`Validating claim ${index + 1}/${claims.length} (row ${rowNumber})`);

        try {
          if (claim.benefit !== '' && claim.category !== '') {
            await claimParamsToCreateClaimOptions(claim, accessToken);
          } else {
            // Fill in the category with any value, just to skip that part of the validation
            const benefit = benefitsWithCategories[0].name;
            const category = benefitsWithCategories[0].categories[0].subcategory_name;

            await claimParamsToCreateClaimOptions(
              { ...claim, benefit, category },
              accessToken,
            );

            console.log(
              chalk.yellow(
                `Claim ${index + 1}/${
                  claims.length
                } (row ${rowNumber}) doesn't have a benefit and/or category. This will have to be inferred using OpenAI when the claims are submitted`,
              ),
            );
          }

          console.log(
            chalk.green(
              `Validated claim ${index + 1}/${claims.length} (row ${rowNumber})`,
            ),
          );
        } catch (e) {
          console.error(
            chalk.red(
              `Error submitting claim ${index + 1}/${claims.length}: ${serializeError(
                e,
              )} (row ${rowNumber})`,
            ),
          );
        }
      }
    }),
  );

export default command;
