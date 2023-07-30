import { readFileSync } from 'fs';
import path from 'path';
import { lookup } from 'mime-types';
export const getCategoriesForBenefitName = async (accessToken, benefitName) => {
    const profile = await getProfile(accessToken);
    const employeeWalletConfiguration = profile.data.employee.employee_wallets
        .filter((benefit) => benefit.is_employee_eligible)
        .find((benefit) => benefit.company_wallet_configuration.wallet_name === benefitName);
    const companyWalletConfiguration = profile.data.company.company_wallet_configurations.find((companyWalletConfiguration) => companyWalletConfiguration.wallet_name === benefitName);
    if (companyWalletConfiguration == null || employeeWalletConfiguration == null) {
        throw new Error(`Could not find benefit with name \`${benefitName}\`.`);
    }
    const returnedCategories = companyWalletConfiguration.categories;
    return returnedCategories.flatMap((category) => {
        return category.subcategories.flatMap((subcategory) => {
            return [
                {
                    category_id: category.id,
                    category_name: category.name,
                    subcategory_name: subcategory.name,
                    subcategory_value: subcategory.value,
                    subcategory_alias: null,
                    benefit_id: employeeWalletConfiguration.id,
                },
                ...subcategory.aliases.map((alias) => ({
                    category_id: category.id,
                    category_name: category.name,
                    subcategory_name: subcategory.name,
                    subcategory_value: subcategory.value,
                    subcategory_alias: alias,
                    benefit_id: employeeWalletConfiguration.id,
                })),
            ];
        });
    });
};
const getProfile = async (accessToken) => {
    const response = await fetch('https://api.joinforma.com/client/api/v3/settings/profile?is_mobile=true', {
        headers: {
            'x-auth-token': accessToken,
        },
    });
    if (!response.ok) {
        throw new Error(`Something went wrong while fetching profile - expected \`200 OK\` response, got \`${response.status} ${response.statusText}\`.`);
    }
    return (await response.json());
};
export const getBenefits = async (accessToken) => {
    const profile = await getProfile(accessToken);
    const { currency: remainingAmountCurrency } = profile.data.employee.settings;
    return profile.data.employee.employee_wallets
        .filter((benefit) => benefit.is_employee_eligible)
        .map((benefit) => ({
        id: benefit.id,
        name: benefit.company_wallet_configuration.wallet_name,
        remainingAmount: benefit.amount,
        remainingAmountCurrency,
    }));
};
export const getBenefitsWithCategories = async (accessToken) => {
    const benefits = await getBenefits(accessToken);
    return await Promise.all(benefits.map(async (benefit) => {
        const categories = await getCategoriesForBenefitName(accessToken, benefit.name);
        return {
            ...benefit,
            categories,
        };
    }));
};
export const createClaim = async (opts) => {
    const { accessToken, amount, merchant, purchaseDate, description, receiptPath, benefitId, categoryId, subcategoryAlias, subcategoryValue, } = opts;
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
    const response = await fetch('https://api.joinforma.com/client/api/v2/claims?is_mobile=true', {
        method: 'POST',
        headers: {
            'x-auth-token': accessToken,
        },
        body: formData,
    });
    if (!response.ok) {
        const responseText = await response.text();
        throw new Error(`Something went wrong while submitting claim - expected \`200 OK\` response, got \`${response.status} ${response.statusText}\`: ${responseText}.`);
    }
    const parsedResponse = (await response.json());
    if (!parsedResponse.success) {
        throw new Error(`Something went wrong while submitting your claim. Received a \`201 Created\` response, but the response body indicated that the request was not successful: ${JSON.stringify(parsedResponse)}.`);
    }
};
export const requestMagicLink = async (email) => {
    const response = await fetch('https://api.joinforma.com/client/auth/v2/login/magic?is_mobile=true', {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
        },
        body: JSON.stringify({ email }),
    });
    if (!response.ok) {
        throw new Error('Unable to request magic link');
    }
    const parsedResponse = (await response.json());
    if (!parsedResponse.success) {
        throw new Error('Unable to request magic link');
    }
};
export const exchangeIdAndTkForAccessToken = async (id, tk) => {
    const requestUrl = new URL('https://api.joinforma.com/client/auth/v2/login/magic');
    requestUrl.search = new URLSearchParams({
        id,
        tk,
        return_token: 'true',
        is_mobile: 'true',
    }).toString();
    const response = await fetch(requestUrl);
    if (!response.ok) {
        throw new Error(`Something went wrong when exchanging the magic link for a token - expected \`200 OK\` response, got \`${response.status} ${response.statusText}\`.`);
    }
    const parsedResponse = (await response.json());
    if (!parsedResponse.success) {
        throw new Error('Something went wrong when exchanging the magic link for a token. Received a `200 OK` response, but the response body indicated that the request was not successful');
    }
    return parsedResponse.data.auth_token;
};
