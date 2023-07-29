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
    const response = await fetch("https://api.joinforma.com/client/api/v3/settings/profile?is_mobile=true", {
        headers: {
            "x-auth-token": accessToken,
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
