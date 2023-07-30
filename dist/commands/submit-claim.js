import * as commander from 'commander';
import { existsSync } from 'fs';
import { actionRunner } from '../utils.js';
import { getAccessToken } from '../config.js';
import { createClaim, getCategoriesForBenefitName } from '../forma.js';
const command = new commander.Command();
const PURCHASE_DATE_REGEX = /^\d{4}-\d{2}-\d{2}$/;
const isValidPurchaseDate = (purchaseDate) => PURCHASE_DATE_REGEX.test(purchaseDate);
const AMOUNT_REGEX = /^\d+(\.\d{2})?$/;
const isValidAmount = (amount) => AMOUNT_REGEX.test(amount);
command
    .name('submit-claim')
    .description('Submit a claim for a Forma benefit')
    .requiredOption('--benefit <benefit>', 'The benefit you are claiming for')
    .requiredOption('--amount <amount>', 'The amount of the claim')
    .requiredOption('--merchant <merchant>', 'The name of the merchant')
    .requiredOption('--category <category>', 'The category of the claim')
    .requiredOption('--purchase-date <purchase-date>', 'The date of the purchase in YYYY-MM-DD format')
    .requiredOption('--description <description>', 'The description of the claim')
    .requiredOption('--receipt-path <receipt-path>', 'The path of the receipt. JPEG, PNG, PDF and HEIC files up to 10MB are accepted.')
    .option('--access_token <access_token>', 'Access token used to authenticate with Forma')
    .action(actionRunner(async (opts) => {
    const accessToken = opts.accessToken ?? getAccessToken();
    if (!accessToken) {
        throw new Error("You aren't logged in to Forma. Please run `formanator login` first.");
    }
    const categories = await getCategoriesForBenefitName(accessToken, opts.benefit);
    const matchingCategory = categories.find((category) => category.subcategory_alias === opts.category ||
        category.subcategory_name === opts.category);
    if (matchingCategory == null) {
        throw new Error(`No category '${opts.category}' found for benefit '${opts.benefit}'.`);
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
}));
export default command;
