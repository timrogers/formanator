import * as commander from 'commander';
import chalk from 'chalk';

import { actionRunner } from '../utils.js';
import { getAccessToken } from '../config.js';
import { createClaim, getBenefitsWithCategories } from '../forma.js';
import { claimParamsToCreateClaimOptions } from '../claims.js';
import VERSION from '../version.js';
import {
  attemptToInferCategoryAndBenefit,
  attemptToInferAllFromReceipt,
} from '../openai.js';

const command = new commander.Command();

interface Arguments {
  accessToken?: string;
  benefit?: string;
  amount?: string;
  merchant?: string;
  category?: string;
  purchaseDate?: string;
  description?: string;
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
  .option(
    '--amount <amount>',
    'The amount of the claim. You may omit this if using receipt inference with OpenAI.',
  )
  .option(
    '--merchant <merchant>',
    'The name of the merchant. You may omit this if using receipt inference with OpenAI.',
  )
  .option(
    '--category <category>',
    'The category of the claim. You may omit this if an OpenAI API key is configured.',
  )
  .option(
    '--purchase-date <purchase-date>',
    'The date of the purchase in YYYY-MM-DD format. You may omit this if using receipt inference with OpenAI.',
  )
  .option(
    '--description <description>',
    'The description of the claim. You may omit this if using receipt inference with OpenAI.',
  )
  .requiredOption(
    '--receipt-path <receipt-path...>',
    'The path of the receipt. JPEG, PNG, PDF and HEIC files up to 10MB are accepted. You may specify this argument multiple times to attach multiple receipts.',
  )
  .option('--access-token <access_token>', 'Access token used to authenticate with Forma')
  .option(
    '--openai-api-key <openai_token>',
    'An optional OpenAI API key used to infer claim details from receipt images, or just the benefit and category based on the merchant and description. If this is set, you may omit other claim details when providing a receipt. This can also be configured using the `OPENAI_API_KEY` environment variable.',
    process.env.OPENAI_API_KEY,
  )
  .option(
    '--github-token <github-token>',
    'An optinoal GitHub Token to use GitHub Models to infer the benefit and category based on the merchant and description. If this is set, you may omit the `--benefit` and `--category` options. This can also be configured using the `GITHUB_TOKEN` environment variable.',
    process.env.GITHUB_TOKEN,
  )
  .action(
    actionRunner(async (opts: Arguments) => {
      const {
        benefit,
        category,
        githubToken,
        openaiApiKey,
        amount,
        merchant,
        purchaseDate,
        description,
      } = opts;

      const accessToken = opts.accessToken ?? getAccessToken();

      if (!accessToken) {
        throw new Error(
          "You aren't logged in to Forma. Please run `formanator login` first.",
        );
      }

      // Check if we have all the manual inputs
      const hasAllManualInputs =
        benefit && category && amount && merchant && purchaseDate && description;

      // Check if we have some manual inputs but not all (mixed mode)
      const hasSomeManualInputs =
        benefit || category || amount || merchant || purchaseDate || description;

      if (hasAllManualInputs) {
        // Traditional mode: all details provided manually
        const createClaimOptions = await claimParamsToCreateClaimOptions(
          {
            benefit: benefit!,
            category: category!,
            amount: amount!,
            merchant: merchant!,
            purchaseDate: purchaseDate!,
            description: description!,
            receiptPath: opts.receiptPath,
          },
          accessToken,
        );
        await createClaim(createClaimOptions);
      } else if (!hasSomeManualInputs && (openaiApiKey || githubToken)) {
        // Receipt inference mode: no manual inputs, just receipt + OpenAI
        const benefitsWithCategories = await getBenefitsWithCategories(accessToken);
        const inferredDetails = await attemptToInferAllFromReceipt({
          receiptPath: opts.receiptPath[0], // Use first receipt for inference
          benefitsWithCategories,
          openaiApiKey,
          githubToken,
        });

        const createClaimOptions = await claimParamsToCreateClaimOptions(
          { ...inferredDetails, receiptPath: opts.receiptPath },
          accessToken,
        );
        await createClaim(createClaimOptions);
      } else if (merchant && description && openaiApiKey) {
        // Legacy mode: infer benefit and category only
        const benefitsWithCategories = await getBenefitsWithCategories(accessToken);
        const { benefit: inferredBenefit, category: inferredCategory } =
          await attemptToInferCategoryAndBenefit({
            merchant: merchant!,
            description: description!,
            benefitsWithCategories,
            openaiApiKey,
          });

        if (!amount || !purchaseDate) {
          throw new Error(
            'When using OpenAI to infer only benefit and category, you must still provide --amount and --purchase-date.',
          );
        }

        const createClaimOptions = await claimParamsToCreateClaimOptions(
          {
            benefit: inferredBenefit,
            category: inferredCategory,
            amount: amount!,
            merchant: merchant!,
            purchaseDate: purchaseDate!,
            description: description!,
            receiptPath: opts.receiptPath,
          },
          accessToken,
        );
        await createClaim(createClaimOptions);
      } else {
        throw new Error(
          'You must either provide all claim details (--benefit, --category, --amount, --merchant, --purchase-date, --description), ' +
            'or provide an OpenAI API key or GitHub token with either: (1) just a receipt for full inference, or (2) --merchant and --description for benefit/category inference.',
        );
      }

      console.log(chalk.green('Claim submitted successfully ✅'));
    }),
  );

export default command;
