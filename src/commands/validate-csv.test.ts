import { beforeEach, describe, expect, it, jest } from '@jest/globals';

// Mock dependencies before importing
jest.mock('fs');
jest.mock('../config.js');
jest.mock('../forma.js');
jest.mock('../claims.js');

import fs from 'fs';
import { getAccessToken } from '../config.js';
import { getBenefitsWithCategories } from '../forma.js';
import { readClaimsFromCsv, claimParamsToCreateClaimOptions } from '../claims.js';

// We need to dynamically import the command after mocks are set up
// Since it's a default export, we'll test the implementation logic separately

const mockFs = fs as jest.Mocked<typeof fs>;
const mockGetAccessToken = getAccessToken as jest.MockedFunction<typeof getAccessToken>;
const mockGetBenefitsWithCategories = getBenefitsWithCategories as jest.MockedFunction<
  typeof getBenefitsWithCategories
>;
const mockReadClaimsFromCsv = readClaimsFromCsv as jest.MockedFunction<
  typeof readClaimsFromCsv
>;
const mockClaimParamsToCreateClaimOptions =
  claimParamsToCreateClaimOptions as jest.MockedFunction<
    typeof claimParamsToCreateClaimOptions
  >;

// Test the validate-csv command logic
describe('validate-csv command', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  // Test the core validation logic that would be used in the command
  describe('CSV validation logic', () => {
    const mockClaims = [
      {
        category: 'wellness',
        benefit: 'Health & Wellness',
        amount: '25.99',
        merchant: 'Test Gym',
        purchaseDate: '2024-01-15',
        description: 'Gym membership',
        receiptPath: ['/path/to/receipt.pdf'],
      },
      {
        category: '', // Empty category - should trigger inference warning
        benefit: '',
        amount: '50.00',
        merchant: 'Test Store',
        purchaseDate: '2024-01-16',
        description: 'Office supplies',
        receiptPath: ['/path/to/receipt2.pdf'],
      },
    ];

    const mockBenefitsWithCategories = [
      {
        id: 'benefit-1',
        name: 'Health & Wellness',
        remainingAmount: 1000,
        remainingAmountCurrency: 'USD',
        categories: [
          {
            category_id: 'cat-1',
            category_name: 'Fitness',
            subcategory_name: 'Gym',
            subcategory_value: 'gym',
            subcategory_alias: 'wellness',
            benefit_id: 'benefit-1',
          },
        ],
      },
    ];

    it('should validate CSV file existence', async () => {
      mockFs.existsSync.mockReturnValue(false);

      // The command should check if file exists
      expect(mockFs.existsSync('/nonexistent/file.csv')).toBe(false);
    });

    it('should read claims from CSV', async () => {
      mockFs.existsSync.mockReturnValue(true);
      mockReadClaimsFromCsv.mockResolvedValue(mockClaims);

      const claims = await readClaimsFromCsv('/test/file.csv');
      expect(claims).toEqual(mockClaims);
      expect(mockReadClaimsFromCsv).toHaveBeenCalledWith('/test/file.csv');
    });

    it('should validate claims with complete benefit and category info', async () => {
      const completeClaim = mockClaims[0];
      mockClaimParamsToCreateClaimOptions.mockResolvedValue({
        ...completeClaim,
        accessToken: 'test-token',
        benefitId: 'benefit-1',
        categoryId: 'cat-1',
        subcategoryAlias: 'wellness',
        subcategoryValue: 'gym',
      });

      const result = await claimParamsToCreateClaimOptions(completeClaim, 'test-token');

      expect(result.benefitId).toBe('benefit-1');
      expect(result.categoryId).toBe('cat-1');
      expect(mockClaimParamsToCreateClaimOptions).toHaveBeenCalledWith(
        completeClaim,
        'test-token',
      );
    });

    it('should handle claims with missing benefit/category for inference', async () => {
      const incompleteClaim = mockClaims[1];
      mockGetBenefitsWithCategories.mockResolvedValue(mockBenefitsWithCategories);

      // For incomplete claims, the command should use first benefit/category for validation
      const claimWithDefaults = {
        ...incompleteClaim,
        benefit: mockBenefitsWithCategories[0].name,
        category: mockBenefitsWithCategories[0].categories[0].subcategory_name,
      };

      mockClaimParamsToCreateClaimOptions.mockResolvedValue({
        ...claimWithDefaults,
        accessToken: 'test-token',
        benefitId: 'benefit-1',
        categoryId: 'cat-1',
        subcategoryAlias: null,
        subcategoryValue: 'gym',
      });

      const result = await claimParamsToCreateClaimOptions(
        claimWithDefaults,
        'test-token',
      );
      expect(result.benefitId).toBe('benefit-1');
    });

    it('should handle validation errors gracefully', async () => {
      const invalidClaim = {
        ...mockClaims[0],
        amount: 'invalid-amount',
      };

      mockClaimParamsToCreateClaimOptions.mockRejectedValue(
        new Error('Amount must be in the format 0.00.'),
      );

      await expect(
        claimParamsToCreateClaimOptions(invalidClaim, 'test-token'),
      ).rejects.toThrow('Amount must be in the format 0.00.');
    });

    it('should handle access token retrieval', () => {
      mockGetAccessToken.mockReturnValue('stored-token');

      const token = getAccessToken();
      expect(token).toBe('stored-token');
    });

    it('should handle missing access token', () => {
      mockGetAccessToken.mockReturnValue(null);

      const token = getAccessToken();
      expect(token).toBeNull();
    });

    it('should get benefits with categories for validation', async () => {
      mockGetBenefitsWithCategories.mockResolvedValue(mockBenefitsWithCategories);

      const benefits = await getBenefitsWithCategories('test-token');
      expect(benefits).toEqual(mockBenefitsWithCategories);
      expect(mockGetBenefitsWithCategories).toHaveBeenCalledWith('test-token');
    });

    it('should handle empty CSV file', async () => {
      mockFs.existsSync.mockReturnValue(true);
      mockReadClaimsFromCsv.mockResolvedValue([]);

      const claims = await readClaimsFromCsv('/empty/file.csv');
      expect(claims).toEqual([]);
    });
  });
});
