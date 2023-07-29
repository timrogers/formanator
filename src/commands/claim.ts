import * as commander from 'commander';
import { existsSync, readFileSync } from 'fs';
import { lookup } from 'mime-types';
import path from 'path';

import { actionRunner } from '../utils.js';
import { getAccessToken } from '../config.js';
import { getCategoriesForBenefitName } from '../forma.js';

const command = new commander.Command();

interface Arguments {
  accessToken?: string;
  benefit: string;
  amount: string;
  merchant: string;
  category: string;
  purchaseDate: string;
  description: string;
  receiptPath: string;
}

interface CreateClaimOptions {
  amount: string;
  merchant: string;
  purchaseDate: string;
  description: string;
  receiptPath: string;
  accessToken: string;
  benefitId: string;
  categoryId: string;
  subcategoryValue: string;
  subcategoryAlias: string | null;
}

const PURCHASE_DATE_REGEX = /^\d{4}-\d{2}-\d{2}$/;

const isValidPurchaseDate = (purchaseDate: string): boolean =>
  PURCHASE_DATE_REGEX.test(purchaseDate);

const AMOUNT_REGEX = /^\d+(\.\d{2})?$/;

const isValidAmount = (amount: string): boolean => AMOUNT_REGEX.test(amount);

const createClaim = async (opts: CreateClaimOptions): Promise<void> => {
  const {
    accessToken,
    amount,
    merchant,
    purchaseDate,
    description,
    receiptPath,
    benefitId,
    categoryId,
    subcategoryAlias,
    subcategoryValue,
  } = opts;

  const formData = new FormData();
  formData.append('type', 'transaction');
  formData.append('is_recurring', 'false');
  formData.append('amount', amount);
  formData.append('transaction_date', purchaseDate);
  formData.append('default_employee_wallet_id', benefitId);
  formData.append('note', description);
  formData.append('category', categoryId);
  formData.append('category_alias', '');
  formData.append('subcategory', subcategoryValue);
  formData.append('subcategory_alias', subcategoryAlias ?? '');
  formData.append('reimbursement_vendor', merchant);

  const receiptData = readFileSync(receiptPath);
  const receiptBlob = new Blob([receiptData], { type: lookup(receiptPath) });
  const receiptFilename = path.basename(receiptPath);

  formData.set('file[]', receiptBlob, receiptFilename);

  const response = await fetch(
    'https://api.joinforma.com/client/api/v2/claims?is_mobile=true',
    {
      method: 'POST',
      headers: {
        'x-auth-token': accessToken,
      },
      body: formData,
    },
  );

  if (!response.ok) {
    const responseText = await response.text();
    throw new Error(
      `Something went wrong while submitting claim - expected \`200 OK\` response, got \`${response.status} ${response.statusText}\`: ${responseText}.`,
    );
  }

  const parsedResponse = (await response.json()) as { success: boolean };

  if (!parsedResponse.success) {
    throw new Error(
      `Something went wrong while submitting your claim. Received a \`201 Created\` response, but the response body indicated that the request was not successful: ${JSON.stringify(
        parsedResponse,
      )}.`,
    );
  }
};

command
  .name('claim')
  .description('Submit a claim for a Forma benefit')
  .requiredOption('--benefit <benefit>', 'The benefit you are claiming for')
  .requiredOption('--amount <amount>', 'The amount of the claim')
  .requiredOption('--merchant <merchant>', 'The name of the merchant')
  .requiredOption('--category <category>', 'The category of the claim')
  .requiredOption(
    '--purchase-date <purchase-date>',
    'The date of the purchase in YYYY-MM-DD format',
  )
  .requiredOption('--description <description>', 'The description of the claim')
  .requiredOption(
    '--receipt-path <receipt-path>',
    'The path of the receipt. JPEG, PNG, PDF and HEIC files up to 10MB are accepted.',
  )
  .option('--access_token <access_token>', 'Access token used to authenticate with Forma')
  .action(
    actionRunner(async (opts: Arguments) => {
      const accessToken = opts.accessToken ?? getAccessToken();

      if (!accessToken) {
        throw new Error(
          "You aren't logged in to Forma. Please run `formanator login` first.",
        );
      }

      const categories = await getCategoriesForBenefitName(accessToken, opts.benefit);

      const matchingCategory = categories.find(
        (category) =>
          category.subcategory_alias === opts.category ||
          category.subcategory_name === opts.category,
      );

      if (matchingCategory == null) {
        throw new Error(
          `No category '${opts.category}' found for benefit '${opts.benefit}'.`,
        );
      }

      if (!isValidPurchaseDate(opts.purchaseDate))
        throw new Error('Purchase date must be in YYYY-MM-DD format.');
      if (!isValidAmount(opts.amount))
        throw new Error('Amount must be in the format 0.00.');
      if (!existsSync(opts.receiptPath))
        throw new Error(`Receipt path '${opts.receiptPath}' does not exist.`);

      await createClaim({
        ...opts,
        accessToken,
        benefitId: matchingCategory.benefit_id,
        categoryId: matchingCategory.category_id,
        subcategoryAlias: matchingCategory.subcategory_alias,
        subcategoryValue: matchingCategory.subcategory_value,
      });

      console.log('Claim submitted successfully.');
    }),
  );

export default command;
