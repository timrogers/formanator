interface Benefit {
  id: string;
  name: string;
  remainingAmount: number;
  remainingAmountCurrency: string;
}

interface ProfileResponseCategory {
  id: string;
  name: string;
  subcategories: Array<{
    name: string;
    value: string;
    aliases: string[];
  }>;
}

interface Category {
  category_id: string;
  category_name: string;
  subcategory_name: string;
  subcategory_value: string;
  subcategory_alias: string | null;
  benefit_id: string;
}

interface ProfileResponse {
  data: {
    company: {
      company_wallet_configurations: Array<{
        id: string;
        wallet_name: string;
        categories: ProfileResponseCategory[];
      }>;
    };
    employee: {
      employee_wallets: Array<{
        id: string;
        amount: number;
        company_wallet_configuration: {
          wallet_name: string;
        };
        is_employee_eligible: boolean;
      }>;
      settings: {
        currency: string;
      };
    };
  };
}

export const getCategoriesForBenefitName = async (
  accessToken: string,
  benefitName: string,
): Promise<Category[]> => {
  const profile = await getProfile(accessToken);

  const employeeWalletConfiguration = profile.data.employee.employee_wallets
    .filter((benefit) => benefit.is_employee_eligible)
    .find((benefit) => benefit.company_wallet_configuration.wallet_name === benefitName);

  const companyWalletConfiguration =
    profile.data.company.company_wallet_configurations.find(
      (companyWalletConfiguration) =>
        companyWalletConfiguration.wallet_name === benefitName,
    );

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

const getProfile = async (accessToken: string): Promise<ProfileResponse> => {
  const response = await fetch(
    'https://api.joinforma.com/client/api/v3/settings/profile?is_mobile=true',
    {
      headers: {
        'x-auth-token': accessToken,
      },
    },
  );

  if (!response.ok) {
    throw new Error(
      `Something went wrong while fetching profile - expected \`200 OK\` response, got \`${response.status} ${response.statusText}\`.`,
    );
  }

  return (await response.json()) as ProfileResponse;
};

export const getBenefits = async (accessToken: string): Promise<Benefit[]> => {
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
