// Mock dependencies to avoid ESM issues in tests
jest.mock('chalk', () => ({
  default: {
    yellow: (msg: string) => msg,
    red: (msg: string) => msg,
    green: (msg: string) => msg,
  },
}));

jest.mock('openai', () => ({
  default: jest.fn(),
}));

jest.mock('fs', () => ({
  readFileSync: jest.fn(),
  extname: jest.fn(),
}));

jest.mock('../utils.js', () => ({
  prompt: jest.fn(),
}));

jest.mock('../forma.js', () => ({}));

import { generateOpenaiPrompt } from '../openai.js';

describe('openai', () => {
  describe('generateOpenaiPrompt', () => {
    // Typical cases
    describe('typical cases', () => {
      it('generates prompt with multiple categories', () => {
        const result = generateOpenaiPrompt({
          validCategories: ['Food & Dining', 'Transportation', 'Office Supplies'],
          merchant: 'Starbucks',
          description: 'Coffee for client meeting',
        });

        expect(result).toContain(
          'Your job is to predict the category for an expense claim',
        );
        expect(result).toContain('Food & Dining');
        expect(result).toContain('Transportation');
        expect(result).toContain('Office Supplies');
        expect(result).toContain('Merchant: Starbucks');
        expect(result).toContain('Description: Coffee for client meeting');
      });

      it('generates prompt with single category', () => {
        const result = generateOpenaiPrompt({
          validCategories: ['Travel'],
          merchant: 'Uber',
          description: 'Ride to airport',
        });

        expect(result).toContain('Travel');
        expect(result).toContain('Merchant: Uber');
        expect(result).toContain('Description: Ride to airport');
        expect(result).not.toContain('Food & Dining');
      });

      it('generates prompt with standard business expense', () => {
        const result = generateOpenaiPrompt({
          validCategories: ['Meals', 'Office Equipment', 'Software'],
          merchant: 'Best Buy',
          description: 'Wireless mouse for home office',
        });

        expect(result).toContain('Meals');
        expect(result).toContain('Office Equipment');
        expect(result).toContain('Software');
        expect(result).toContain('Merchant: Best Buy');
        expect(result).toContain('Description: Wireless mouse for home office');
      });
    });

    // Edge cases
    describe('edge cases', () => {
      it('handles empty categories array', () => {
        const result = generateOpenaiPrompt({
          validCategories: [],
          merchant: 'Test Merchant',
          description: 'Test description',
        });

        expect(result).toContain('Your job is to predict the category');
        expect(result).toContain('Here are the possible categories:');
        expect(result).toContain('Merchant: Test Merchant');
        expect(result).toContain('Description: Test description');
        // Should not have any category lines between the headers
        const lines = result.split('\n');
        const categoriesStart = lines.findIndex((line) =>
          line.includes('Here are the possible categories:'),
        );
        const predictStart = lines.findIndex((line) =>
          line.includes('Please predict the category'),
        );
        const categoryLines = lines
          .slice(categoriesStart + 1, predictStart)
          .filter((line) => line.trim() !== '');
        expect(categoryLines).toHaveLength(0);
      });

      it('handles empty merchant string', () => {
        const result = generateOpenaiPrompt({
          validCategories: ['Food'],
          merchant: '',
          description: 'Some purchase',
        });

        expect(result).toContain('Merchant: ');
        expect(result).toContain('Description: Some purchase');
      });

      it('handles empty description string', () => {
        const result = generateOpenaiPrompt({
          validCategories: ['Travel'],
          merchant: 'Airline',
          description: '',
        });

        expect(result).toContain('Merchant: Airline');
        expect(result).toContain('Description: ');
      });

      it('handles special characters in merchant name', () => {
        const result = generateOpenaiPrompt({
          validCategories: ['Food'],
          merchant: "Joe's Café & Bistro (Downtown)",
          description: 'Business lunch',
        });

        expect(result).toContain("Merchant: Joe's Café & Bistro (Downtown)");
        expect(result).toContain('Description: Business lunch');
      });

      it('handles special characters in description', () => {
        const result = generateOpenaiPrompt({
          validCategories: ['Software'],
          merchant: 'Microsoft',
          description: 'Office 365 subscription - $99.99/month (annual plan)',
        });

        expect(result).toContain('Merchant: Microsoft');
        expect(result).toContain(
          'Description: Office 365 subscription - $99.99/month (annual plan)',
        );
      });

      it('handles categories with special characters', () => {
        const result = generateOpenaiPrompt({
          validCategories: ['Meals & Entertainment', 'IT/Software', 'R&D Equipment'],
          merchant: 'Tech Store',
          description: 'Equipment purchase',
        });

        expect(result).toContain('Meals & Entertainment');
        expect(result).toContain('IT/Software');
        expect(result).toContain('R&D Equipment');
      });

      it('handles very long merchant name', () => {
        const longMerchant = 'A'.repeat(200);
        const result = generateOpenaiPrompt({
          validCategories: ['Office'],
          merchant: longMerchant,
          description: 'Test purchase',
        });

        expect(result).toContain(`Merchant: ${longMerchant}`);
      });

      it('handles very long description', () => {
        const longDescription =
          'This is a very long description that goes on and on and contains lots of details about the purchase including the date, time, location, specific items purchased, and many other details that might be relevant to categorizing this expense claim properly. '.repeat(
            5,
          );
        const result = generateOpenaiPrompt({
          validCategories: ['Office'],
          merchant: 'Office Depot',
          description: longDescription,
        });

        expect(result).toContain(`Description: ${longDescription}`);
      });

      it('handles large number of categories', () => {
        const manyCategories = Array.from({ length: 50 }, (_, i) => `Category ${i + 1}`);
        const result = generateOpenaiPrompt({
          validCategories: manyCategories,
          merchant: 'Test Merchant',
          description: 'Test description',
        });

        // Check that all categories are included
        manyCategories.forEach((category) => {
          expect(result).toContain(category);
        });
      });

      it('handles whitespace in inputs', () => {
        const result = generateOpenaiPrompt({
          validCategories: ['  Food  ', '\tTravel\t', '\nOffice\n'],
          merchant: '  Starbucks  ',
          description: '\tCoffee purchase\t',
        });

        expect(result).toContain('  Food  ');
        expect(result).toContain('\tTravel\t');
        expect(result).toContain('\nOffice\n');
        expect(result).toContain('Merchant:   Starbucks  ');
        expect(result).toContain('Description: \tCoffee purchase\t');
      });
    });

    // Structure and format validation
    describe('prompt structure and format', () => {
      it('has correct overall structure', () => {
        const result = generateOpenaiPrompt({
          validCategories: ['Food', 'Travel'],
          merchant: 'Test Merchant',
          description: 'Test description',
        });

        // Check main sections are present
        expect(result).toContain(
          'Your job is to predict the category for an expense claim',
        );
        expect(result).toContain('Here are the possible categories:');
        expect(result).toContain('Please predict the category for the following claim:');
        expect(result).toContain('Merchant: Test Merchant');
        expect(result).toContain('Description: Test description');
      });

      it('formats categories with newlines', () => {
        const result = generateOpenaiPrompt({
          validCategories: ['Food', 'Travel', 'Office'],
          merchant: 'Test',
          description: 'Test',
        });

        // Categories should be on separate lines
        expect(result).toMatch(/Food\nTravel\nOffice/);
      });

      it('maintains consistent spacing', () => {
        const result = generateOpenaiPrompt({
          validCategories: ['Food'],
          merchant: 'Test',
          description: 'Test',
        });

        // Check for proper spacing around sections
        expect(result).toMatch(/categories:\n\nFood\n\nPlease predict/);
        expect(result).toMatch(/claim:\n\nMerchant: Test\nDescription: Test$/);
      });

      it('does not add extra punctuation', () => {
        const result = generateOpenaiPrompt({
          validCategories: ['Food'],
          merchant: 'Test',
          description: 'Test',
        });

        // The prompt should end with the description, no extra punctuation
        expect(result).toMatch(/Description: Test$/);
        expect(result).not.toMatch(/Description: Test\.$/);
        expect(result).not.toMatch(/Description: Test!$/);
      });
    });

    // Return value validation
    describe('return value validation', () => {
      it('always returns a string', () => {
        const result = generateOpenaiPrompt({
          validCategories: ['Food'],
          merchant: 'Test',
          description: 'Test',
        });

        expect(typeof result).toBe('string');
        expect(result.length).toBeGreaterThan(0);
      });

      it('returns non-empty string even with minimal input', () => {
        const result = generateOpenaiPrompt({
          validCategories: [],
          merchant: '',
          description: '',
        });

        expect(typeof result).toBe('string');
        expect(result.length).toBeGreaterThan(50); // Should have at least the instruction text
      });
    });
  });
});
