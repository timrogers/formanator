import axios from 'axios';
import { createReadStream } from 'fs';
import { validateAxiosStatus } from './utils.js';
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
    const response = await axios.get('https://api.joinforma.com/client/api/v3/settings/profile?is_mobile=true', {
        headers: {
            'x-auth-token': accessToken,
        },
        validateStatus: validateAxiosStatus,
    });
    if (response.status !== 200) {
        throw new Error(`Something went wrong while fetching profile - expected \`200 OK\` response, got \`${response.status} ${response.statusText}\`.`);
    }
    return response.data;
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
    const response = await axios.post('https://api.joinforma.com/client/api/v2/claims?is_mobile=true', {
        type: 'transaction',
        is_recurring: 'false',
        amount,
        transaction_date: purchaseDate,
        default_employee_wallet_id: benefitId,
        note: description,
        category: categoryId,
        category_alias: '',
        subcategory: subcategoryValue,
        subcategory_alias: subcategoryAlias ?? '',
        reimbursement_vendor: merchant,
        file: [createReadStream(receiptPath)],
    }, {
        headers: {
            'Content-Type': 'multipart/form-data',
            'x-auth-token': accessToken,
        },
    });
    if (response.status !== 201) {
        throw new Error(`Something went wrong while submitting claim - expected \`201 Created\` response, got \`${response.status} ${response.statusText}\`.`);
    }
    const parsedResponse = response.data;
    if (!parsedResponse.success) {
        throw new Error(`Something went wrong while submitting your claim. Received a \`201 Created\` response, but the response body indicated that the request was not successful: ${JSON.stringify(parsedResponse)}.`);
    }
};
export const requestMagicLink = async (email) => {
    const response = await axios.post('https://api.joinforma.com/client/auth/v2/login/magic?is_mobile=true', { email });
    if (response.status !== 200) {
        throw new Error(`Something went wrong while requesting magic link - expected \`200 OK\` response, got \`${response.status} ${response.statusText}\`.`);
    }
    const parsedResponse = response.data;
    if (!parsedResponse.success) {
        throw new Error(`Something went wrong while requesting magic link - received a \`200 OK\` response, but the response body indicated that the request was not successful: ${JSON.stringify(parsedResponse)}`);
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
    const response = await axios.get(requestUrl.toString());
    if (response.status !== 200) {
        throw new Error(`Something went wrong while exchanging magic link for token - expected \`200 OK\` response, got \`${response.status} ${response.statusText}\`.`);
    }
    const parsedResponse = response.data;
    if (!parsedResponse.success) {
        throw new Error(`Something went wrong while exchanging magic link for token - received a \`200 OK\` response, but the response body indicated that the request was not successful: ${JSON.stringify(parsedResponse)}`);
    }
    return parsedResponse.data.auth_token;
};
