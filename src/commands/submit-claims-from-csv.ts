import * as commander from 'commander';
import { createReadStream, existsSync } from 'fs';
import { parse } from '@fast-csv/parse';
import chalk from 'chalk';

import { actionRunner, serializeError } from '../utils.js';
import { getAccessToken } from '../config.js';
import { createClaim, getBenefitsWithCategories } from '../forma.js';
import { type Claim, claimParamsToCreateClaimOptions } from '../claims.js';
import VERSION from '../version.js';
import { attemptToInferCategoryAndBenefit } from '../openai.js';

const command = new commander.Command();

interface Arguments {
  accessToken?: string;
  inputPath: string;
  openaiApiKey: string;
}

const EXPECTED_HEADERS = [
  'category',
  'benefit',
  'amount',
  'merchant',
  'purchaseDate',
  'description',
  'receiptPath',
];

const readClaimsFromCsv = async (inputPath: string): Promise<Claim[]> => {
  const claims: Claim[] = [];

  return await new Promise((resolve, reject) => {
    createReadStream(inputPath, 'utf8')
      .pipe(parse({ headers: true }))
      .on('error', reject)
      .on('data', (row) => {
        const rowHeaders = Object.keys(row);

        if (
          rowHeaders.length !== EXPECTED_HEADERS.length ||
          !rowHeaders.every((header) => EXPECTED_HEADERS.includes(header))
        ) {
          reject(
            new Error(
              'Invalid CSV headers. Please use a template CSV generated by the `generate-template-csv` command.',
            ),
          );
        }

        const receiptPath = row.receiptPath.split(',').map((path) => path.trim());
        const claim: Claim = { ...row, receiptPath };

        claims.push(claim);
      })
      .on('end', () => {
        resolve(claims);
      });
  });
};

command
  .name('submit-claims-from-csv')
  .version(VERSION)
  .description(
    'Submit multiple Forma claims from a CSV. To generate a template CSV to fill in, use the `generate-template-csv` command. You may attach multiple receipts to a claim by filling the `receipt_path` column with comma-separated paths.',
  )
  .requiredOption('--input-path <input_path>', 'The path to the CSV to read claims from')
  .option('--access-token <access_token>', 'Access token used to authenticate with Forma')
  .option(
    '--openai-api-key <openai_token>',
    'An optional OpenAI API key used to infer the benefit and category based on the merchant and description. If this is set, you may omit the `--benefit` and `--category` options. This can also be configured using the `OPENAI_API_KEY` environment variable.',
    process.env.OPENAI_API_KEY,
  )
  .action(
    actionRunner(async (opts: Arguments) => {
      const { openaiApiKey } = opts;

      const accessToken = opts.accessToken ?? getAccessToken();

      if (!accessToken) {
        throw new Error(
          "You aren't logged in to Forma. Please run `formanator login` first.",
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

      for (const [index, claim] of claims.entries()) {
        console.log(`Submitting claim ${index + 1}/${claims.length}`);

        try {
          if (claim.benefit !== '' && claim.category !== '') {
            const createClaimOptions = await claimParamsToCreateClaimOptions(
              claim,
              accessToken,
            );
            await createClaim(createClaimOptions);
            console.log(
              chalk.green(`Successfully submitted claim ${index + 1}/${claims.length}`),
            );
          } else if (openaiApiKey) {
            const benefitsWithCategories = await getBenefitsWithCategories(accessToken);
            const { benefit, category } = await attemptToInferCategoryAndBenefit({
              merchant: claim.merchant,
              description: claim.description,
              benefitsWithCategories,
              openaiApiKey,
            });

            const createClaimOptions = await claimParamsToCreateClaimOptions(
              { ...claim, benefit, category },
              accessToken,
            );
            await createClaim(createClaimOptions);
            console.log(
              chalk.green(`Successfully submitted claim ${index + 1}/${claims.length}`),
            );
          } else {
            throw new Error(
              'You must either fill out the `benefit` and `category` columns, or specify an OpenAI API key.',
            );
          }
        } catch (e) {
          console.error(
            chalk.red(
              `Error submitting claim ${index + 1}/${claims.length}: ${serializeError(
                e,
              )}`,
            ),
          );
        }
      }
    }),
  );

export default command;
