import axios from 'axios';
import { createReadStream } from 'fs';

import { validateAxiosStatus } from './utils.js';

interface Benefit {
  id: string;
  name: string;
  remainingAmount: number;
  remainingAmountCurrency: string;
}

export interface BenefitWithCategories extends Benefit {
  categories: Category[];
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

export interface CreateClaimOptions {
  amount: string;
  merchant: string;
  purchaseDate: string;
  description: string;
  receiptPath: string[];
  accessToken: string;
  benefitId: string;
  categoryId: string;
  subcategoryValue: string;
  subcategoryAlias: string | null;
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
  const response = await axios.get(
    'https://api.joinforma.com/client/api/v3/settings/profile?is_mobile=true',
    {
      headers: {
        'x-auth-token': accessToken,
      },
      validateStatus: validateAxiosStatus,
    },
  );

  if (response.status !== 200) {
    throw new Error(
      `Something went wrong while fetching profile - expected \`200 OK\` response, got \`${response.status} ${response.statusText}\`.`,
    );
  }

  return response.data as ProfileResponse;
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

export const getBenefitsWithCategories = async (
  accessToken: string,
): Promise<BenefitWithCategories[]> => {
  const benefits = await getBenefits(accessToken);

  return await Promise.all(
    benefits.map(async (benefit) => {
      const categories = await getCategoriesForBenefitName(accessToken, benefit.name);

      return {
        ...benefit,
        categories,
      };
    }),
  );
};

interface CreateClaimResponse {
  success: boolean;
}

export const createClaim = async (opts: CreateClaimOptions): Promise<void> => {
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

  const data = {
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
    file: receiptPath.map((path) => createReadStream(path)),
  };

  const response = await axios.post(
    'https://api.joinforma.com/client/api/v2/claims?is_mobile=true',
    data,
    {
      headers: {
        'Content-Type': 'multipart/form-data',
        'x-auth-token': accessToken,
      },
    },
  );

  if (response.status !== 201) {
    throw new Error(
      `Something went wrong while submitting claim - expected \`201 Created\` response, got \`${response.status} ${response.statusText}\`.`,
    );
  }

  const parsedResponse = response.data as CreateClaimResponse;

  if (!parsedResponse.success) {
    throw new Error(
      `Something went wrong while submitting your claim. Received a \`201 Created\` response, but the response body indicated that the request was not successful: ${JSON.stringify(
        parsedResponse,
      )}.`,
    );
  }
};

interface RequestMagicLinkResponse {
  success: boolean;
  status: number;
  data: {
    done: boolean;
  };
}

export const requestMagicLink = async (email: string): Promise<void> => {
  const response = await axios.post(
    'https://api.joinforma.com/client/auth/v2/login/magic?is_mobile=true',
    { email },
  );

  if (response.status !== 200) {
    throw new Error(
      `Something went wrong while requesting magic link - expected \`200 OK\` response, got \`${response.status} ${response.statusText}\`.`,
    );
  }

  const parsedResponse = response.data as RequestMagicLinkResponse;

  if (!parsedResponse.success) {
    throw new Error(
      `Something went wrong while requesting magic link - received a \`200 OK\` response, but the response body indicated that the request was not successful: ${JSON.stringify(
        parsedResponse,
      )}`,
    );
  }
};

interface ExchangeIdAndTkForAccessTokenResponse {
  success: boolean;
  status: number;
  data: { auth_token: string };
}

export const exchangeIdAndTkForAccessToken = async (
  id: string,
  tk: string,
): Promise<string> => {
  const requestUrl = new URL('https://api.joinforma.com/client/auth/v2/login/magic');

  requestUrl.search = new URLSearchParams({
    id,
    tk,
    return_token: 'true',
    is_mobile: 'true',
  }).toString();

  const response = await axios.get(requestUrl.toString());

  if (response.status !== 200) {
    throw new Error(
      `Something went wrong while exchanging magic link for token - expected \`200 OK\` response, got \`${response.status} ${response.statusText}\`.`,
    );
  }

  const parsedResponse = response.data as ExchangeIdAndTkForAccessTokenResponse;

  if (!parsedResponse.success) {
    throw new Error(
      `Something went wrong while exchanging magic link for token - received a \`200 OK\` response, but the response body indicated that the request was not successful: ${JSON.stringify(
        parsedResponse,
      )}`,
    );
  }

  return parsedResponse.data.auth_token;
};
