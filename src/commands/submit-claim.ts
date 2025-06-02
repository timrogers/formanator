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
    'The benefit you are claiming for. Optional if an OpenAI API key or GitHub token is provided, in which case this will be inferred from the receipt or merchant/description.',
  )
  .option(
    '--amount <amount>',
    'The amount of the claim. Optional if an OpenAI API key or GitHub token is provided, in which case this will be inferred from the receipt.',
  )
  .option(
    '--merchant <merchant>',
    'The name of the merchant. Optional if an OpenAI API key or GitHub token is provided, in which case this will be inferred from the receipt.',
  )
  .option(
    '--category <category>',
    'The category of the claim. Optional if an OpenAI API key or GitHub token is provided, in which case this will be inferred from the receipt or merchant/description.',
  )
  .option(
    '--purchase-date <purchase-date>',
    'The date of purchase in YYYY-MM-DD format. Optional if an OpenAI API key or GitHub token is provided, in which case this will be inferred from the receipt.',
  )
  .option(
    '--description <description>',
    'The description of the claim. Optional if an OpenAI API key or GitHub token is provided, in which case this will be inferred from the receipt.',
  )
  .requiredOption(
    '--receipt-path <receipt-path...>',
    'The path of the receipt. JPEG, PNG, PDF and HEIC files up to 10MB are accepted. You may specify this argument multiple times to attach multiple receipts.',
  )
  .option('--access-token <access_token>', 'Access token used to authenticate with Forma')
  .option(
    '--openai-api-key <openai_token>',
    'An optional OpenAI API key used to infer claim details. If this or a GitHub token is provided, you may omit (a) omit the benefit and category and allow them to be inferred by the LLM or (b) omit everything except the receipt path, and allow all details to be inferred by the LLM. This can also be configured using the `OPENAI_API_KEY` environment variable.',
    process.env.OPENAI_API_KEY,
  )
  .option(
    '--github-token <github-token>',
    'An optional GitHub token used to infer claim details. If this or an OpenAI API key is provided, you may omit (a) omit the benefit and category and allow them to be inferred by the LLM or (b) omit everything except the receipt path, and allow all details to be inferred by the LLM. This can also be configured using the `GITHUB_TOKEN` environment variable.',
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

      const hasLlmInferenceKey = openaiApiKey || githubToken;

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
      } else if (!hasSomeManualInputs && hasLlmInferenceKey) {
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
            githubToken,
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
            'or provide an OpenAI API key or GitHub token with either: (1) just a receipt for full inference, or (2) all details except --benefit and --category to infer them.',
        );
      }

      console.log(chalk.green('Claim submitted successfully ✅'));
    }),
  );

export default command;
