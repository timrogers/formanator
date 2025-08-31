import { beforeEach, describe, expect, it, jest } from '@jest/globals';

// Mock dependencies before importing
jest.mock('axios');
jest.mock('fs');
jest.mock('../src/utils.js', () => ({
  serializeError: jest.fn((e) =>
    typeof e === 'string' ? e : e instanceof Error ? e.message : JSON.stringify(e),
  ),
}));

import axios from 'axios';
import fs from 'fs';
import {
  getCategoriesForBenefitName,
  getBenefits,
  getBenefitsWithCategories,
  getClaimsList,
  createClaim,
  requestMagicLink,
  exchangeIdAndTkForAccessToken,
  handleErrorResponse,
  validateAxiosStatus,
  type CreateClaimOptions,
} from '../src/forma.js';

const mockAxios = axios as jest.Mocked<typeof axios>;
const mockFs = fs as jest.Mocked<typeof fs>;

// Define a type for mock error responses - removing unused interface
// interface MockErrorResponse {
//   status: number;
//   statusText: string;
//   data: unknown;
//   headers: Record<string, string>;
//   config: Record<string, unknown>;
// }

describe('forma', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  describe('validateAxiosStatus', () => {
    it('should always return true', () => {
      expect(validateAxiosStatus()).toBe(true);
    });
  });

  describe('handleErrorResponse', () => {
    it('should throw mapped error message for known errors', () => {
      const response = {
        status: 401,
        statusText: 'Unauthorized',
        headers: {},
        config: {},
        data: {
          success: false,
          data: null,
          errors: { message: 'JWT token is invalid' },
          message: 'JWT token is invalid',
          status: 401,
        },
      };

      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      expect(() => handleErrorResponse(response as unknown as any)).toThrow(
        'Your Forma access token is invalid. Please log in again with `npx formanator login`.',
      );
    });

    it('should throw original error message for unmapped errors', () => {
      const response = {
        status: 400,
        statusText: 'Bad Request',
        headers: {},
        config: {},
        data: {
          success: false,
          data: null,
          errors: { message: 'Invalid request' },
          message: 'Invalid request',
          status: 400,
        },
      };

      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      expect(() => handleErrorResponse(response as unknown as any)).toThrow(
        'Invalid request',
      );
    });

    it('should throw generic error for unparseable responses', () => {
      const response = {
        status: 500,
        statusText: 'Internal Server Error',
        headers: {},
        config: {},
        data: 'Some unexpected response format',
      };

      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      expect(() => handleErrorResponse(response as unknown as any)).toThrow(
        'Received an unexpected 500 Internal Server Error response from Forma: Some unexpected response format',
      );
    });
  });

  describe('getBenefits', () => {
    const mockProfileResponse = {
      status: 200,
      data: {
        data: {
          employee: {
            employee_wallets: [
              {
                id: 'wallet-1',
                amount: 500,
                company_wallet_configuration: { wallet_name: 'Health & Wellness' },
                is_employee_eligible: true,
              },
              {
                id: 'wallet-2',
                amount: 1000,
                company_wallet_configuration: { wallet_name: 'Learning & Development' },
                is_employee_eligible: true,
              },
              {
                id: 'wallet-3',
                amount: 200,
                company_wallet_configuration: { wallet_name: 'Ineligible Benefit' },
                is_employee_eligible: false,
              },
            ],
            settings: { currency: 'USD' },
          },
        },
      },
    };

    it('should return only eligible benefits', async () => {
      mockAxios.get.mockResolvedValue(mockProfileResponse);

      const result = await getBenefits('test-token');

      expect(result).toEqual([
        {
          id: 'wallet-1',
          name: 'Health & Wellness',
          remainingAmount: 500,
          remainingAmountCurrency: 'USD',
        },
        {
          id: 'wallet-2',
          name: 'Learning & Development',
          remainingAmount: 1000,
          remainingAmountCurrency: 'USD',
        },
      ]);

      expect(mockAxios.get).toHaveBeenCalledWith(
        'https://api.joinforma.com/client/api/v3/settings/profile',
        {
          headers: { 'x-auth-token': 'test-token' },
          validateStatus: validateAxiosStatus,
        },
      );
    });

    it('should handle error responses', async () => {
      const errorResponse = {
        status: 401,
        data: {
          success: false,
          errors: { message: 'JWT token is invalid' },
          message: 'JWT token is invalid',
          status: 401,
        },
      };

      mockAxios.get.mockResolvedValue(errorResponse);

      await expect(getBenefits('invalid-token')).rejects.toThrow(
        'Your Forma access token is invalid. Please log in again with `npx formanator login`.',
      );
    });
  });

  describe('getCategoriesForBenefitName', () => {
    const mockProfileResponse = {
      status: 200,
      data: {
        data: {
          employee: {
            employee_wallets: [
              {
                id: 'wallet-1',
                company_wallet_configuration: { wallet_name: 'Health & Wellness' },
                is_employee_eligible: true,
              },
            ],
          },
          company: {
            company_wallet_configurations: [
              {
                id: 'config-1',
                wallet_name: 'Health & Wellness',
                categories: [
                  {
                    id: 'cat-1',
                    name: 'Fitness',
                    subcategories: [
                      {
                        name: 'Gym Membership',
                        value: 'gym',
                        aliases: ['fitness', 'workout'],
                      },
                    ],
                  },
                ],
              },
            ],
          },
        },
      },
    };

    it('should return categories with aliases for existing benefit', async () => {
      mockAxios.get.mockResolvedValue(mockProfileResponse);

      const result = await getCategoriesForBenefitName('test-token', 'Health & Wellness');

      expect(result).toEqual([
        {
          category_id: 'cat-1',
          category_name: 'Fitness',
          subcategory_name: 'Gym Membership',
          subcategory_value: 'gym',
          subcategory_alias: null,
          benefit_id: 'wallet-1',
        },
        {
          category_id: 'cat-1',
          category_name: 'Fitness',
          subcategory_name: 'Gym Membership',
          subcategory_value: 'gym',
          subcategory_alias: 'fitness',
          benefit_id: 'wallet-1',
        },
        {
          category_id: 'cat-1',
          category_name: 'Fitness',
          subcategory_name: 'Gym Membership',
          subcategory_value: 'gym',
          subcategory_alias: 'workout',
          benefit_id: 'wallet-1',
        },
      ]);
    });

    it('should throw error for non-existent benefit', async () => {
      mockAxios.get.mockResolvedValue(mockProfileResponse);

      await expect(
        getCategoriesForBenefitName('test-token', 'Non-existent Benefit'),
      ).rejects.toThrow('Could not find benefit with name `Non-existent Benefit`.');
    });
  });

  describe('getClaimsList', () => {
    const mockClaimsResponse = {
      status: 200,
      data: {
        data: {
          claims: [
            {
              id: 'claim-1',
              status: 'approved',
              reimbursement: {
                status: 'processed',
                payout_status: 'paid',
                amount: 25.99,
                category: 'Fitness',
                subcategory: 'Gym',
                reimbursement_vendor: 'Test Gym',
                date_processed: '2024-01-15',
                note: 'Monthly membership',
                employee_note: 'Gym subscription',
              },
            },
          ],
        },
      },
    };

    it('should return transformed claims list', async () => {
      mockAxios.get.mockResolvedValue(mockClaimsResponse);

      const result = await getClaimsList('test-token', undefined);

      expect(result).toEqual([
        {
          id: 'claim-1',
          status: 'approved',
          reimbursement_status: 'processed',
          payout_status: 'paid',
          amount: 25.99,
          category: 'Fitness',
          subcategory: 'Gym',
          reimbursement_vendor: 'Test Gym',
          date_processed: '2024-01-15',
          note: 'Monthly membership',
          employee_note: 'Gym subscription',
        },
      ]);

      expect(mockAxios.get).toHaveBeenCalledWith(
        'https://api.joinforma.com/client/api/v2/claims?page=0',
        {
          headers: { 'x-auth-token': 'test-token' },
          validateStatus: validateAxiosStatus,
        },
      );
    });
  });

  describe('createClaim', () => {
    const mockCreateClaimOptions: CreateClaimOptions = {
      accessToken: 'test-token',
      amount: '25.99',
      merchant: 'Test Store',
      purchaseDate: '2024-01-15',
      description: 'Test purchase',
      receiptPath: ['/path/to/receipt.pdf'],
      benefitId: 'wallet-1',
      categoryId: 'cat-1',
      subcategoryValue: 'gym',
      subcategoryAlias: 'fitness',
    };

    it('should create claim successfully', async () => {
      const mockResponse = { status: 201, data: { success: true } };
      mockAxios.post.mockResolvedValue(mockResponse);
      (
        mockFs.createReadStream as jest.MockedFunction<typeof mockFs.createReadStream>
      ).mockReturnValue('mock-stream' as unknown as fs.ReadStream);

      await createClaim(mockCreateClaimOptions);

      expect(mockAxios.post).toHaveBeenCalledWith(
        'https://api.joinforma.com/client/api/v2/claims',
        {
          type: 'transaction',
          is_recurring: 'false',
          amount: '25.99',
          transaction_date: '2024-01-15',
          default_employee_wallet_id: 'wallet-1',
          note: 'Test purchase',
          category: 'cat-1',
          category_alias: '',
          subcategory: 'gym',
          subcategory_alias: 'fitness',
          reimbursement_vendor: 'Test Store',
          file: ['mock-stream'],
        },
        {
          headers: {
            'Content-Type': 'multipart/form-data',
            'x-auth-token': 'test-token',
          },
        },
      );
    });

    it('should handle subcategory_alias null value', async () => {
      const mockResponse = { status: 201, data: { success: true } };
      mockAxios.post.mockResolvedValue(mockResponse);
      (
        mockFs.createReadStream as jest.MockedFunction<typeof mockFs.createReadStream>
      ).mockReturnValue('mock-stream' as unknown as fs.ReadStream);

      const optionsWithNullAlias = { ...mockCreateClaimOptions, subcategoryAlias: null };
      await createClaim(optionsWithNullAlias);

      const postCall = mockAxios.post.mock.calls[0];
      expect((postCall[1] as { subcategory_alias: string }).subcategory_alias).toBe('');
    });

    it('should throw error when response indicates failure', async () => {
      const mockResponse = { status: 201, data: { success: false } };
      mockAxios.post.mockResolvedValue(mockResponse);
      (
        mockFs.createReadStream as jest.MockedFunction<typeof mockFs.createReadStream>
      ).mockReturnValue('mock-stream' as unknown as fs.ReadStream);

      await expect(createClaim(mockCreateClaimOptions)).rejects.toThrow(
        'Something went wrong while submitting your claim. Received a `201 Created` response, but the response body indicated that the request was not successful: {"success":false}.',
      );
    });

    it('should handle non-201 response status', async () => {
      const mockResponse = {
        status: 400,
        data: {
          success: false,
          errors: { message: 'Invalid data' },
          message: 'Invalid data',
          status: 400,
        },
      };
      mockAxios.post.mockResolvedValue(mockResponse);
      (
        mockFs.createReadStream as jest.MockedFunction<typeof mockFs.createReadStream>
      ).mockReturnValue('mock-stream' as unknown as fs.ReadStream);

      await expect(createClaim(mockCreateClaimOptions)).rejects.toThrow('Invalid data');
    });
  });

  describe('requestMagicLink', () => {
    it('should request magic link successfully', async () => {
      const mockResponse = {
        status: 200,
        data: { success: true, status: 200, data: { done: true } },
      };
      mockAxios.post.mockResolvedValue(mockResponse);

      await requestMagicLink('test@example.com');

      expect(mockAxios.post).toHaveBeenCalledWith(
        'https://api.joinforma.com/client/auth/v2/login/magic',
        { email: 'test@example.com' },
        { validateStatus: validateAxiosStatus },
      );
    });

    it('should throw error when success is false', async () => {
      const mockResponse = {
        status: 200,
        data: { success: false, status: 200, data: { done: false } },
      };
      mockAxios.post.mockResolvedValue(mockResponse);

      await expect(requestMagicLink('test@example.com')).rejects.toThrow(
        'Something went wrong while requesting magic link - received a `200 OK` response, but the response body indicated that the request was not successful: {"success":false,"status":200,"data":{"done":false}}',
      );
    });
  });

  describe('exchangeIdAndTkForAccessToken', () => {
    it('should exchange tokens successfully', async () => {
      const mockResponse = {
        status: 200,
        data: { success: true, status: 200, data: { auth_token: 'new-access-token' } },
      };
      mockAxios.get.mockResolvedValue(mockResponse);

      const result = await exchangeIdAndTkForAccessToken('test-id', 'test-tk');

      expect(result).toBe('new-access-token');
      expect(mockAxios.get).toHaveBeenCalledWith(
        'https://api.joinforma.com/client/auth/v2/login/magic?id=test-id&tk=test-tk&return_token=true',
      );
    });

    it('should throw error when success is false', async () => {
      const mockResponse = {
        status: 200,
        data: { success: false, status: 200, data: {} },
      };
      mockAxios.get.mockResolvedValue(mockResponse);

      await expect(exchangeIdAndTkForAccessToken('test-id', 'test-tk')).rejects.toThrow(
        'Something went wrong while exchanging magic link for token - received a `200 OK` response, but the response body indicated that the request was not successful: {"success":false,"status":200,"data":{}}',
      );
    });
  });

  describe('getBenefitsWithCategories', () => {
    it('should combine benefits with their categories', async () => {
      // Mock the profile response for getBenefits
      const mockProfileResponse = {
        status: 200,
        data: {
          data: {
            employee: {
              employee_wallets: [
                {
                  id: 'wallet-1',
                  amount: 500,
                  company_wallet_configuration: { wallet_name: 'Health & Wellness' },
                  is_employee_eligible: true,
                },
              ],
              settings: { currency: 'USD' },
            },
            company: {
              company_wallet_configurations: [
                {
                  id: 'config-1',
                  wallet_name: 'Health & Wellness',
                  categories: [
                    {
                      id: 'cat-1',
                      name: 'Fitness',
                      subcategories: [{ name: 'Gym', value: 'gym', aliases: [] }],
                    },
                  ],
                },
              ],
            },
          },
        },
      };

      mockAxios.get.mockResolvedValue(mockProfileResponse);

      const result = await getBenefitsWithCategories('test-token');

      expect(result).toEqual([
        {
          id: 'wallet-1',
          name: 'Health & Wellness',
          remainingAmount: 500,
          remainingAmountCurrency: 'USD',
          categories: [
            {
              category_id: 'cat-1',
              category_name: 'Fitness',
              subcategory_name: 'Gym',
              subcategory_value: 'gym',
              subcategory_alias: null,
              benefit_id: 'wallet-1',
            },
          ],
        },
      ]);
    });
  });
});
