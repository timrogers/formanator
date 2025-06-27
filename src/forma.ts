import axios from 'axios';
import { createReadStream } from 'fs';

import { validateAxiosStatus, checkFor403Error } from './utils.js';
import {
  ProfileResponseSchema,
  ClaimsListResponseSchema,
  CreateClaimResponseSchema,
  RequestMagicLinkResponseSchema,
  ExchangeIdAndTkForAccessTokenResponseSchema,
  type ProfileResponse,
  type ClaimsListResponse,
  type CreateClaimResponse,
  type RequestMagicLinkResponse,
  type ExchangeIdAndTkForAccessTokenResponse,
} from './schemas.js';

interface Benefit {
  id: string;
  name: string;
  remainingAmount: number;
  remainingAmountCurrency: string;
}

export interface BenefitWithCategories extends Benefit {
  categories: Category[];
}

interface Category {
  category_id: string;
  category_name: string;
  subcategory_name: string;
  subcategory_value: string;
  subcategory_alias: string | null;
  benefit_id: string;
}

interface Claim {
  id: string;
  status: string;
  reimbursement_status: string;
  payout_status: string;
  amount: number;
  category: string;
  subcategory: string;
  reimbursement_vendor: string;
  date_processed: string;
  note: string;
  employee_note: string;
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
    'https://api.joinforma.com/client/api/v3/settings/profile',
    {
      headers: {
        'x-auth-token': accessToken,
      },
      validateStatus: validateAxiosStatus,
    },
  );

  if (response.status !== 200) {
    checkFor403Error(response.status);
    throw new Error(
      `Something went wrong while fetching profile - expected \`200 OK\` response, got \`${response.status} ${response.statusText}\`.`,
    );
  }

  try {
    return ProfileResponseSchema.parse(response.data);
  } catch (error) {
    throw new Error(
      `Invalid profile response format: ${error instanceof Error ? error.message : 'Unknown validation error'}`,
    );
  }
};

const getClaims = async (
  accessToken: string,
  page: number = 0,
): Promise<ClaimsListResponse> => {
  const response = await axios.get(
    `https://api.joinforma.com/client/api/v2/claims?page=${page}`,
    {
      headers: {
        'x-auth-token': accessToken,
      },
      validateStatus: validateAxiosStatus,
    },
  );

  if (response.status !== 200) {
    checkFor403Error(response.status);
    throw new Error(
      `Something went wrong while fetching claims - expected \`200 OK\` response, got \`${response.status} ${response.statusText}\`.`,
    );
  }

  try {
    return ClaimsListResponseSchema.parse(response.data);
  } catch (error) {
    throw new Error(
      `Invalid claims response format: ${error instanceof Error ? error.message : 'Unknown validation error'}`,
    );
  }
};

export const getClaimsList = async (
  accessToken: string,
  page: number,
): Promise<Claim[]> => {
  const claims = await getClaims(accessToken, page);
  return claims.data.claims.map((claim) => ({
    id: claim.id,
    status: claim.status,
    reimbursement_status: claim.reimbursement.status,
    payout_status: claim.reimbursement.payout_status,
    amount: claim.reimbursement.amount,
    category: claim.reimbursement.category,
    subcategory: claim.reimbursement.subcategory,
    reimbursement_vendor: claim.reimbursement.reimbursement_vendor,
    date_processed: claim.reimbursement.date_processed,
    note: claim.reimbursement.note,
    employee_note: claim.reimbursement.employee_note,
  }));
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
    'https://api.joinforma.com/client/api/v2/claims',
    data,
    {
      headers: {
        'Content-Type': 'multipart/form-data',
        'x-auth-token': accessToken,
      },
    },
  );

  if (response.status !== 201) {
    checkFor403Error(response.status);
    throw new Error(
      `Something went wrong while submitting claim - expected \`201 Created\` response, got \`${response.status} ${response.statusText}\`.`,
    );
  }

  let parsedResponse: CreateClaimResponse;
  try {
    parsedResponse = CreateClaimResponseSchema.parse(response.data);
  } catch (error) {
    throw new Error(
      `Invalid create claim response format: ${error instanceof Error ? error.message : 'Unknown validation error'}`,
    );
  }

  if (!parsedResponse.success) {
    throw new Error(
      `Something went wrong while submitting your claim. Received a \`201 Created\` response, but the response body indicated that the request was not successful: ${JSON.stringify(
        parsedResponse,
      )}.`,
    );
  }
};

export const requestMagicLink = async (email: string): Promise<void> => {
  const response = await axios.post(
    'https://api.joinforma.com/client/auth/v2/login/magic',
    { email },
  );

  if (response.status !== 200) {
    checkFor403Error(response.status);
    throw new Error(
      `Something went wrong while requesting magic link - expected \`200 OK\` response, got \`${response.status} ${response.statusText}\`.`,
    );
  }

  let parsedResponse: RequestMagicLinkResponse;
  try {
    parsedResponse = RequestMagicLinkResponseSchema.parse(response.data);
  } catch (error) {
    throw new Error(
      `Invalid magic link response format: ${error instanceof Error ? error.message : 'Unknown validation error'}`,
    );
  }

  if (!parsedResponse.success) {
    throw new Error(
      `Something went wrong while requesting magic link - received a \`200 OK\` response, but the response body indicated that the request was not successful: ${JSON.stringify(
        parsedResponse,
      )}`,
    );
  }
};

export const exchangeIdAndTkForAccessToken = async (
  id: string,
  tk: string,
): Promise<string> => {
  const requestUrl = new URL('https://api.joinforma.com/client/auth/v2/login/magic');

  requestUrl.search = new URLSearchParams({
    id,
    tk,
    return_token: 'true',
  }).toString();

  const response = await axios.get(requestUrl.toString());

  if (response.status !== 200) {
    checkFor403Error(response.status);
    throw new Error(
      `Something went wrong while exchanging magic link for token - expected \`200 OK\` response, got \`${response.status} ${response.statusText}\`.`,
    );
  }

  let parsedResponse: ExchangeIdAndTkForAccessTokenResponse;
  try {
    parsedResponse = ExchangeIdAndTkForAccessTokenResponseSchema.parse(response.data);
  } catch (error) {
    throw new Error(
      `Invalid token exchange response format: ${error instanceof Error ? error.message : 'Unknown validation error'}`,
    );
  }

  if (!parsedResponse.success) {
    throw new Error(
      `Something went wrong while exchanging magic link for token - received a \`200 OK\` response, but the response body indicated that the request was not successful: ${JSON.stringify(
        parsedResponse,
      )}`,
    );
  }

  return parsedResponse.data.auth_token;
};
