import * as commander from 'commander';
import { readdirSync, existsSync, mkdirSync, renameSync } from 'fs';
import { join, extname, basename } from 'path';
import chalk from 'chalk';

import { actionRunner, prompt } from '../utils.js';
import { getAccessToken } from '../config.js';
import { createClaim, getBenefitsWithCategories } from '../forma.js';
import { claimParamsToCreateClaimOptions } from '../claims.js';
import VERSION from '../version.js';
import { attemptToInferAllFromReceipt } from '../openai.js';

const command = new commander.Command();

interface Arguments {
  accessToken?: string;
  directory: string;
  processedDirectory?: string;
  openaiApiKey?: string;
  githubToken?: string;
}

// Supported file extensions for receipts
const SUPPORTED_EXTENSIONS = ['.jpg', '.jpeg', '.png', '.pdf', '.heic'];

// Helper function to check if a file is a supported receipt type
const isSupportedReceiptFile = (filename: string): boolean => {
  const ext = extname(filename).toLowerCase();
  return SUPPORTED_EXTENSIONS.includes(ext);
};

// Helper function to get all receipt files from directory
const getReceiptFiles = (directory: string): string[] => {
  if (!existsSync(directory)) {
    throw new Error(`Directory '${directory}' does not exist.`);
  }

  const files = readdirSync(directory);
  return files.filter(isSupportedReceiptFile).map((file) => join(directory, file));
};

// Helper function to move file to processed directory
const moveFileToProcessed = (sourceFile: string, processedDir: string): void => {
  if (!existsSync(processedDir)) {
    mkdirSync(processedDir, { recursive: true });
  }

  const filename = basename(sourceFile);
  const destinationFile = join(processedDir, filename);

  // If destination file already exists, add a timestamp suffix
  let finalDestination = destinationFile;
  if (existsSync(destinationFile)) {
    const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
    const ext = extname(filename);
    const nameWithoutExt = basename(filename, ext);
    finalDestination = join(processedDir, `${nameWithoutExt}-${timestamp}${ext}`);
  }

  renameSync(sourceFile, finalDestination);
  console.log(chalk.blue(`Moved processed receipt to: ${finalDestination}`));
};

command
  .name('submit-receipts-from-directory')
  .version(VERSION)
  .description('Submit claims from all receipt files in a directory')
  .requiredOption(
    '--directory <directory>',
    'The directory containing receipt files to process',
  )
  .option(
    '--processed-directory <processed-directory>',
    'Directory to move successfully processed receipts (defaults to processed/ subdirectory)',
  )
  .option('--access-token <access_token>', 'Access token used to authenticate with Forma')
  .option(
    '--openai-api-key <openai_token>',
    'An optional OpenAI API key used to infer claim details from receipt images. This can also be configured using the `OPENAI_API_KEY` environment variable.',
    process.env.OPENAI_API_KEY,
  )
  .option(
    '--github-token <github-token>',
    'An optional GitHub Token to use GitHub Models to infer claim details from receipt images. This can also be configured using the `GITHUB_TOKEN` environment variable.',
    process.env.GITHUB_TOKEN,
  )
  .action(
    actionRunner(async (opts: Arguments) => {
      const { directory, openaiApiKey, githubToken } = opts;

      const accessToken = opts.accessToken ?? getAccessToken();

      if (!accessToken) {
        throw new Error(
          "You aren't logged in to Forma. Please run `formanator login` first.",
        );
      }

      if (!openaiApiKey && !githubToken) {
        throw new Error(
          'You must provide either an OpenAI API key (--openai-api-key) or GitHub token (--github-token) to infer claim details from receipts.',
        );
      }

      // Set up processed directory
      const processedDirectory = opts.processedDirectory ?? join(directory, 'processed');

      // Get all receipt files from the directory
      const receiptFiles = getReceiptFiles(directory);

      if (receiptFiles.length === 0) {
        console.log(
          chalk.yellow(`No supported receipt files found in directory: ${directory}`),
        );
        console.log(
          chalk.yellow(`Supported file types: ${SUPPORTED_EXTENSIONS.join(', ')}`),
        );
        return;
      }

      console.log(
        chalk.green(`Found ${receiptFiles.length} receipt file(s) to process:`),
      );
      receiptFiles.forEach((file, index) => {
        console.log(`  ${index + 1}. ${basename(file)}`);
      });
      console.log();

      // Get benefits and categories for inference
      const benefitsWithCategories = await getBenefitsWithCategories(accessToken);

      let processedCount = 0;
      let skippedCount = 0;

      // Process each receipt file
      for (const [index, receiptFile] of receiptFiles.entries()) {
        const filename = basename(receiptFile);
        console.log(
          chalk.cyan(
            `\n--- Processing receipt ${index + 1}/${receiptFiles.length}: ${filename} ---`,
          ),
        );

        try {
          // Infer claim details from receipt
          console.log('Analyzing receipt...');
          const inferredDetails = await attemptToInferAllFromReceipt({
            receiptPath: receiptFile,
            benefitsWithCategories,
            openaiApiKey,
            githubToken,
          });

          // Display inferred details to user
          console.log(chalk.green('\nInferred claim details:'));
          console.log(`  Amount: ${chalk.yellow(inferredDetails.amount)}`);
          console.log(`  Merchant: ${chalk.yellow(inferredDetails.merchant)}`);
          console.log(`  Purchase Date: ${chalk.yellow(inferredDetails.purchaseDate)}`);
          console.log(`  Description: ${chalk.yellow(inferredDetails.description)}`);
          console.log(`  Benefit: ${chalk.yellow(inferredDetails.benefit)}`);
          console.log(`  Category: ${chalk.yellow(inferredDetails.category)}`);

          // Prompt user for confirmation
          console.log(
            chalk.white(
              '\nDo you want to submit this claim? Enter Y to proceed or N to skip:',
            ),
          );
          const userResponse = prompt('> ').trim().toLowerCase();

          if (userResponse === 'y' || userResponse === 'yes') {
            // Submit the claim
            console.log('Submitting claim...');
            const createClaimOptions = await claimParamsToCreateClaimOptions(
              { ...inferredDetails, receiptPath: [receiptFile] },
              accessToken,
            );
            await createClaim(createClaimOptions);

            console.log(chalk.green(`✅ Claim submitted successfully for ${filename}`));

            // Move file to processed directory
            moveFileToProcessed(receiptFile, processedDirectory);
            processedCount++;
          } else {
            console.log(chalk.yellow(`Skipped ${filename}`));
            skippedCount++;
          }
        } catch (error) {
          console.error(
            chalk.red(
              `❌ Error processing ${filename}: ${error instanceof Error ? error.message : String(error)}`,
            ),
          );
          skippedCount++;
        }
      }

      // Summary
      console.log(chalk.green(`\n--- Summary ---`));
      console.log(`Processed successfully: ${chalk.green(processedCount)}`);
      console.log(`Skipped: ${chalk.yellow(skippedCount)}`);
      console.log(`Total files: ${receiptFiles.length}`);

      if (processedCount > 0) {
        console.log(chalk.blue(`Processed receipts moved to: ${processedDirectory}`));
      }
    }),
  );

export default command;
