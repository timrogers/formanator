import { existsSync } from 'fs';
import { getCategoriesForBenefitName } from './forma.js';
const PURCHASE_DATE_REGEX = /^\d{4}-\d{2}-\d{2}$/;
const isValidPurchaseDate = (purchaseDate) => PURCHASE_DATE_REGEX.test(purchaseDate);
const AMOUNT_REGEX = /^\d+(\.\d{2})?$/;
const isValidAmount = (amount) => AMOUNT_REGEX.test(amount);
export const claimParamsToCreateClaimOptions = async (claim, accessToken) => {
    const categories = await getCategoriesForBenefitName(accessToken, claim.benefit);
    const matchingCategory = categories.find((category) => category.subcategory_alias === claim.category ||
        category.subcategory_name === claim.category);
    if (matchingCategory == null) {
        throw new Error(`No category '${claim.category}' found for benefit '${claim.benefit}'.`);
    }
    if (!isValidPurchaseDate(claim.purchaseDate))
        throw new Error('Purchase date must be in YYYY-MM-DD format.');
    if (!isValidAmount(claim.amount))
        throw new Error('Amount must be in the format 0.00.');
    if (!existsSync(claim.receiptPath))
        throw new Error(`Receipt path '${claim.receiptPath}' does not exist.`);
    return {
        ...claim,
        accessToken,
        benefitId: matchingCategory.benefit_id,
        categoryId: matchingCategory.category_id,
        subcategoryAlias: matchingCategory.subcategory_alias,
        subcategoryValue: matchingCategory.subcategory_value,
    };
};
