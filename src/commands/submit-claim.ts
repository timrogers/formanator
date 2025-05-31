import * as commander from 'commander';
import chalk from 'chalk';

import { actionRunner } from '../utils.js';
import { getAccessToken } from '../config.js';
import { createClaim, getBenefitsWithCategories } from '../forma.js';
import { claimParamsToCreateClaimOptions } from '../claims.js';
import VERSION from '../version.js';
import { attemptToInferCategoryAndBenefit } from '../openai.js';

const command = new commander.Command();

interface Arguments {
  accessToken?: string;
  benefit?: string;
  amount: string;
  merchant: string;
  category?: string;
  purchaseDate: string;
  description: string;
  receiptPath: string[];
  openaiApiKey?: string;
  githubToken?: string;
}

command
  .name('submit-claim')
  .version(VERSION)
  .description('Submit a claim for a Forma benefit')
  .option(
    '--benefit <benefit>',
    'The benefit you are claiming for. You may omit this if an OpenAI API key is configured',
  )
  .requiredOption('--amount <amount>', 'The amount of the claim')
  .requiredOption('--merchant <merchant>', 'The name of the merchant')
  .option(
    '--category <category>',
    'The category of the claim. You may omit this if an OpenAI API key is configured.',
  )
  .requiredOption(
    '--purchase-date <purchase-date>',
    'The date of the purchase in YYYY-MM-DD format',
  )
  .requiredOption('--description <description>', 'The description of the claim')
  .requiredOption(
    '--receipt-path <receipt-path...>',
    'The path of the receipt. JPEG, PNG, PDF and HEIC files up to 10MB are accepted. You may specify this argument multiple times to attach multiple receipts.',
  )
  .option('--access-token <access_token>', 'Access token used to authenticate with Forma')
  .option(
    '--openai-api-key <openai_token>',
    'An optional OpenAI API key used to infer the benefit and category based on the merchant and description. If this is set, you may omit the `--benefit` and `--category` options. This can also be configured using the `OPENAI_API_KEY` environment variable.',
    process.env.OPENAI_API_KEY,
  )
  .option(
    '--github-token <github-token>',
    'An optinoal GitHub Token to use GitHub Models to infer the benefit and category based on the merchant and description. If this is set, you may omit the `--benefit` and `--category` options. This can also be configured using the `GITHUB_TOKEN` environment variable.',
    process.env.GITHUB_TOKEN,
  )
  .action(
    actionRunner(async (opts: Arguments) => {
      const { benefit, category, openaiApiKey, githubToken } = opts;

      const accessToken = opts.accessToken ?? getAccessToken();

      if (!accessToken) {
        throw new Error(
          "You aren't logged in to Forma. Please run `formanator login` first.",
        );
      }

      if (benefit && category) {
        const createClaimOptions = await claimParamsToCreateClaimOptions(
          { ...opts, benefit, category },
          accessToken,
        );
        await createClaim(createClaimOptions);
      } else if (openaiApiKey || githubToken) {
        const benefitsWithCategories = await getBenefitsWithCategories(accessToken);
        const { benefit, category } = await attemptToInferCategoryAndBenefit({
          merchant: opts.merchant,
          description: opts.description,
          benefitsWithCategories,
          openaiApiKey,
          githubToken,
        });

        const createClaimOptions = await claimParamsToCreateClaimOptions(
          { ...opts, benefit, category },
          accessToken,
        );
        await createClaim(createClaimOptions);
      } else {
        throw new Error(
          'You must either specify --benefit and --category, GitHub Token, or an OpenAI API key.',
        );
      }

      console.log(chalk.green('Claim submitted successfully ✅'));
    }),
  );

export default command;
