import { beforeEach, describe, expect, it, jest } from '@jest/globals';

// Mock dependencies before importing
jest.mock('fs');
jest.mock('../src/forma.js', () => ({
  getCategoriesForBenefitName: jest.fn(),
}));
jest.mock('@fast-csv/parse', () => ({
  parse: jest.fn(),
}));

import fs from 'fs';
import { parse } from '@fast-csv/parse';
import { getCategoriesForBenefitName } from '../src/forma.js';
import {
  claimParamsToCreateClaimOptions,
  readClaimsFromCsv,
  type Claim,
} from '../src/claims.js';

const mockFs = fs as jest.Mocked<typeof fs>;
const mockParse = parse as jest.MockedFunction<typeof parse>;
const mockGetCategoriesForBenefitName =
  getCategoriesForBenefitName as jest.MockedFunction<typeof getCategoriesForBenefitName>;

describe('claims', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  describe('claimParamsToCreateClaimOptions', () => {
    const mockCategories = [
      {
        category_id: 'cat-1',
        category_name: 'Health',
        subcategory_name: 'Wellness',
        subcategory_value: 'wellness',
        subcategory_alias: 'health-wellness',
        benefit_id: 'benefit-1',
      },
      {
        category_id: 'cat-2',
        category_name: 'Education',
        subcategory_name: 'Books',
        subcategory_value: 'books',
        subcategory_alias: null,
        benefit_id: 'benefit-1',
      },
    ];

    const validClaim: Claim = {
      category: 'health-wellness',
      benefit: 'Wellness Benefit',
      amount: '25.99',
      merchant: 'Test Merchant',
      purchaseDate: '2024-01-15',
      description: 'Test purchase',
      receiptPath: ['/path/to/receipt.pdf'],
    };

    beforeEach(() => {
      mockGetCategoriesForBenefitName.mockResolvedValue(mockCategories);
      mockFs.existsSync.mockReturnValue(true);
    });

    it('should transform valid claim with alias match', async () => {
      const result = await claimParamsToCreateClaimOptions(validClaim, 'test-token');

      expect(result).toEqual({
        ...validClaim,
        accessToken: 'test-token',
        benefitId: 'benefit-1',
        categoryId: 'cat-1',
        subcategoryAlias: 'health-wellness',
        subcategoryValue: 'wellness',
      });
    });

    it('should transform valid claim with subcategory name match', async () => {
      const claimWithSubcategoryName = { ...validClaim, category: 'Books' };

      const result = await claimParamsToCreateClaimOptions(
        claimWithSubcategoryName,
        'test-token',
      );

      expect(result).toEqual({
        ...claimWithSubcategoryName,
        accessToken: 'test-token',
        benefitId: 'benefit-1',
        categoryId: 'cat-2',
        subcategoryAlias: null,
        subcategoryValue: 'books',
      });
    });

    it('should throw error for invalid category', async () => {
      const invalidClaim = { ...validClaim, category: 'nonexistent-category' };

      await expect(
        claimParamsToCreateClaimOptions(invalidClaim, 'test-token'),
      ).rejects.toThrow(
        "No category 'nonexistent-category' found for benefit 'Wellness Benefit'.",
      );
    });

    it('should throw error for invalid purchase date format', async () => {
      const invalidClaim = { ...validClaim, purchaseDate: '01/15/2024' };

      await expect(
        claimParamsToCreateClaimOptions(invalidClaim, 'test-token'),
      ).rejects.toThrow('Purchase date must be in YYYY-MM-DD format.');
    });

    it('should throw error for invalid amount format', async () => {
      const invalidClaim = { ...validClaim, amount: '$25.99' };

      await expect(
        claimParamsToCreateClaimOptions(invalidClaim, 'test-token'),
      ).rejects.toThrow('Amount must be in the format 0.00.');
    });

    it('should throw error for non-existent receipt path', async () => {
      mockFs.existsSync.mockReturnValue(false);

      await expect(
        claimParamsToCreateClaimOptions(validClaim, 'test-token'),
      ).rejects.toThrow("Receipt path '/path/to/receipt.pdf' does not exist.");
    });

    it('should validate all receipt paths when multiple receipts', async () => {
      const claimWithMultipleReceipts = {
        ...validClaim,
        receiptPath: ['/path/to/receipt1.pdf', '/path/to/receipt2.pdf'],
      };

      mockFs.existsSync.mockReturnValueOnce(true).mockReturnValueOnce(false);

      await expect(
        claimParamsToCreateClaimOptions(claimWithMultipleReceipts, 'test-token'),
      ).rejects.toThrow("Receipt path '/path/to/receipt2.pdf' does not exist.");
    });

    it('should accept valid amount formats', async () => {
      const testCases = [
        { amount: '25', expected: true },
        { amount: '25.99', expected: true },
        { amount: '0.00', expected: true },
        { amount: '1000.50', expected: true },
      ];

      for (const testCase of testCases) {
        const claim = { ...validClaim, amount: testCase.amount };
        const result = await claimParamsToCreateClaimOptions(claim, 'test-token');
        expect(result.amount).toBe(testCase.amount);
      }
    });

    it('should accept valid date formats', async () => {
      const testCases = ['2024-01-01', '2024-12-31', '2000-06-15'];

      for (const testDate of testCases) {
        const claim = { ...validClaim, purchaseDate: testDate };
        const result = await claimParamsToCreateClaimOptions(claim, 'test-token');
        expect(result.purchaseDate).toBe(testDate);
      }
    });
  });

  describe('readClaimsFromCsv', () => {
    let mockParseInstance: {
      on: jest.Mock;
      pipe: jest.Mock;
    };

    beforeEach(() => {
      mockParseInstance = {
        on: jest.fn().mockReturnThis(),
        pipe: jest.fn().mockReturnThis(),
      };
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      mockParse.mockReturnValue(mockParseInstance as unknown as any);

      // Mock createReadStream to return a simple object
      (
        mockFs.createReadStream as jest.MockedFunction<typeof mockFs.createReadStream>
      ).mockReturnValue({
        pipe: jest.fn().mockReturnValue(mockParseInstance),
      } as unknown as fs.ReadStream);
    });

    it('should parse valid CSV and return claims', async () => {
      const mockCsvData = [
        {
          category: 'health',
          benefit: 'Wellness',
          amount: '25.99',
          merchant: 'Test Store',
          purchaseDate: '2024-01-15',
          description: 'Test item',
          receiptPath: '/path/to/receipt.pdf, /path/to/receipt2.pdf',
        },
      ];

      // Mock the CSV parsing process
      mockParseInstance.on.mockImplementation(
        (event: string, callback: (data?: unknown) => void) => {
          if (event === 'data') {
            mockCsvData.forEach((row) => callback(row));
          } else if (event === 'end') {
            callback();
          }
          return mockParseInstance;
        },
      );

      const result = await readClaimsFromCsv('/test/path.csv');

      expect(result).toHaveLength(1);
      expect(result[0]).toEqual({
        category: 'health',
        benefit: 'Wellness',
        amount: '25.99',
        merchant: 'Test Store',
        purchaseDate: '2024-01-15',
        description: 'Test item',
        receiptPath: ['/path/to/receipt.pdf', '/path/to/receipt2.pdf'],
      });
    });

    it('should reject on invalid CSV headers', async () => {
      const mockCsvDataWithWrongHeaders = [
        {
          wrongHeader1: 'value1',
          wrongHeader2: 'value2',
        },
      ];

      mockParseInstance.on.mockImplementation(
        (event: string, callback: (data?: unknown) => void) => {
          if (event === 'data') {
            callback(mockCsvDataWithWrongHeaders[0]);
          } else if (event === 'error') {
            // Don't call this
          }
          return mockParseInstance;
        },
      );

      await expect(readClaimsFromCsv('/test/path.csv')).rejects.toThrow(
        'Invalid CSV headers. Please use a template CSV generated by the `generate-template-csv` command.',
      );
    });

    it('should reject on parse error', async () => {
      const parseError = new Error('CSV parse error');

      mockParseInstance.on.mockImplementation(
        (event: string, callback: (error?: Error) => void) => {
          if (event === 'error') {
            callback(parseError);
          }
          return mockParseInstance;
        },
      );

      await expect(readClaimsFromCsv('/test/path.csv')).rejects.toThrow(
        'CSV parse error',
      );
    });

    it('should handle receipt paths with whitespace', async () => {
      const mockCsvData = [
        {
          category: 'health',
          benefit: 'Wellness',
          amount: '25.99',
          merchant: 'Test Store',
          purchaseDate: '2024-01-15',
          description: 'Test item',
          receiptPath: ' /path/to/receipt.pdf , /path/to/receipt2.pdf ',
        },
      ];

      mockParseInstance.on.mockImplementation(
        (event: string, callback: (data?: unknown) => void) => {
          if (event === 'data') {
            callback(mockCsvData[0]);
          } else if (event === 'end') {
            callback();
          }
          return mockParseInstance;
        },
      );

      const result = await readClaimsFromCsv('/test/path.csv');

      expect(result[0].receiptPath).toEqual([
        '/path/to/receipt.pdf',
        '/path/to/receipt2.pdf',
      ]);
    });
  });
});
