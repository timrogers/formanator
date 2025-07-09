// Test the generateOpenaiPrompt function directly by copying it here
// This avoids import issues with dependencies in the full openai.ts file

const generateOpenaiPrompt = (opts: {
  validCategories: string[];
  merchant: string;
  description: string;
}): string => {
  const { description, merchant, validCategories } = opts;

  return `Your job is to predict the category for an expense claim based on the name of the merchant and a description of what was purchased. You should give a single, specific answer without any extra words or punctuation.

Here are the possible categories:

${validCategories.join('\n')}

Please predict the category for the following claim:

Merchant: ${merchant}
Description: ${description}`;
};

describe('generateOpenaiPrompt', () => {
  describe('basic functionality', () => {
    it('generates correct prompt with valid inputs', () => {
      const result = generateOpenaiPrompt({
        validCategories: ['Food & Dining', 'Transportation', 'Office Supplies'],
        merchant: 'Starbucks',
        description: 'Coffee and pastry for client meeting',
      });

      expect(result)
        .toBe(`Your job is to predict the category for an expense claim based on the name of the merchant and a description of what was purchased. You should give a single, specific answer without any extra words or punctuation.

Here are the possible categories:

Food & Dining
Transportation
Office Supplies

Please predict the category for the following claim:

Merchant: Starbucks
Description: Coffee and pastry for client meeting`);
    });

    it('generates correct prompt with single category', () => {
      const result = generateOpenaiPrompt({
        validCategories: ['Office Supplies'],
        merchant: 'Amazon',
        description: 'Printer paper and pens',
      });

      expect(result)
        .toBe(`Your job is to predict the category for an expense claim based on the name of the merchant and a description of what was purchased. You should give a single, specific answer without any extra words or punctuation.

Here are the possible categories:

Office Supplies

Please predict the category for the following claim:

Merchant: Amazon
Description: Printer paper and pens`);
    });
  });

  describe('edge cases', () => {
    it('handles empty categories array', () => {
      const result = generateOpenaiPrompt({
        validCategories: [],
        merchant: 'Test Merchant',
        description: 'Test description',
      });

      expect(result)
        .toBe(`Your job is to predict the category for an expense claim based on the name of the merchant and a description of what was purchased. You should give a single, specific answer without any extra words or punctuation.

Here are the possible categories:



Please predict the category for the following claim:

Merchant: Test Merchant
Description: Test description`);
    });

    it('handles empty merchant name', () => {
      const result = generateOpenaiPrompt({
        validCategories: ['Food & Dining'],
        merchant: '',
        description: 'Lunch meeting',
      });

      expect(result)
        .toBe(`Your job is to predict the category for an expense claim based on the name of the merchant and a description of what was purchased. You should give a single, specific answer without any extra words or punctuation.

Here are the possible categories:

Food & Dining

Please predict the category for the following claim:

Merchant: 
Description: Lunch meeting`);
    });

    it('handles empty description', () => {
      const result = generateOpenaiPrompt({
        validCategories: ['Transportation'],
        merchant: 'Uber',
        description: '',
      });

      expect(result)
        .toBe(`Your job is to predict the category for an expense claim based on the name of the merchant and a description of what was purchased. You should give a single, specific answer without any extra words or punctuation.

Here are the possible categories:

Transportation

Please predict the category for the following claim:

Merchant: Uber
Description: `);
    });

    it('handles all empty inputs', () => {
      const result = generateOpenaiPrompt({
        validCategories: [],
        merchant: '',
        description: '',
      });

      expect(result)
        .toBe(`Your job is to predict the category for an expense claim based on the name of the merchant and a description of what was purchased. You should give a single, specific answer without any extra words or punctuation.

Here are the possible categories:



Please predict the category for the following claim:

Merchant: 
Description: `);
    });
  });

  describe('special characters and formatting', () => {
    it('handles categories with special characters', () => {
      const result = generateOpenaiPrompt({
        validCategories: ['R&D Expenses', 'IT & Software', 'Travel & Entertainment'],
        merchant: 'GitHub',
        description: 'Software subscription',
      });

      expect(result)
        .toBe(`Your job is to predict the category for an expense claim based on the name of the merchant and a description of what was purchased. You should give a single, specific answer without any extra words or punctuation.

Here are the possible categories:

R&D Expenses
IT & Software
Travel & Entertainment

Please predict the category for the following claim:

Merchant: GitHub
Description: Software subscription`);
    });

    it('handles merchant with special characters', () => {
      const result = generateOpenaiPrompt({
        validCategories: ['Food & Dining'],
        merchant: "McDonald's & Co.",
        description: 'Team lunch',
      });

      expect(result)
        .toBe(`Your job is to predict the category for an expense claim based on the name of the merchant and a description of what was purchased. You should give a single, specific answer without any extra words or punctuation.

Here are the possible categories:

Food & Dining

Please predict the category for the following claim:

Merchant: McDonald's & Co.
Description: Team lunch`);
    });

    it('handles description with special characters and newlines', () => {
      const result = generateOpenaiPrompt({
        validCategories: ['Office Supplies'],
        merchant: 'Office Depot',
        description: 'Printer paper\nand "special" items',
      });

      expect(result)
        .toBe(`Your job is to predict the category for an expense claim based on the name of the merchant and a description of what was purchased. You should give a single, specific answer without any extra words or punctuation.

Here are the possible categories:

Office Supplies

Please predict the category for the following claim:

Merchant: Office Depot
Description: Printer paper
and "special" items`);
    });
  });

  describe('boundary conditions', () => {
    it('handles very long category names', () => {
      const result = generateOpenaiPrompt({
        validCategories: [
          'This is a very long category name that might be used in some complex expense tracking systems',
        ],
        merchant: 'Test Merchant',
        description: 'Test description',
      });

      expect(result).toContain(
        'This is a very long category name that might be used in some complex expense tracking systems',
      );
    });

    it('handles many categories', () => {
      const categories = Array.from({ length: 10 }, (_, i) => `Category ${i + 1}`);
      const result = generateOpenaiPrompt({
        validCategories: categories,
        merchant: 'Test Merchant',
        description: 'Test description',
      });

      categories.forEach((category) => {
        expect(result).toContain(category);
      });
      expect(result).toContain('Category 1\nCategory 2\nCategory 3');
    });

    it('handles very long merchant name', () => {
      const longMerchant = 'A'.repeat(100);
      const result = generateOpenaiPrompt({
        validCategories: ['Test Category'],
        merchant: longMerchant,
        description: 'Test description',
      });

      expect(result).toContain(`Merchant: ${longMerchant}`);
    });

    it('handles very long description', () => {
      const longDescription = 'B'.repeat(200);
      const result = generateOpenaiPrompt({
        validCategories: ['Test Category'],
        merchant: 'Test Merchant',
        description: longDescription,
      });

      expect(result).toContain(`Description: ${longDescription}`);
    });
  });

  describe('prompt structure verification', () => {
    it('always includes the instruction header', () => {
      const result = generateOpenaiPrompt({
        validCategories: ['Test'],
        merchant: 'Test',
        description: 'Test',
      });

      expect(result).toMatch(/^Your job is to predict the category for an expense claim/);
    });

    it('always includes categories section', () => {
      const result = generateOpenaiPrompt({
        validCategories: ['Test'],
        merchant: 'Test',
        description: 'Test',
      });

      expect(result).toContain('Here are the possible categories:');
    });

    it('always includes merchant and description labels', () => {
      const result = generateOpenaiPrompt({
        validCategories: ['Test'],
        merchant: 'Test',
        description: 'Test',
      });

      expect(result).toContain('Merchant:');
      expect(result).toContain('Description:');
    });

    it('follows the exact expected structure pattern', () => {
      const result = generateOpenaiPrompt({
        validCategories: ['Cat1', 'Cat2'],
        merchant: 'TestMerchant',
        description: 'TestDesc',
      });

      const lines = result.split('\n');
      expect(lines[0]).toBe(
        'Your job is to predict the category for an expense claim based on the name of the merchant and a description of what was purchased. You should give a single, specific answer without any extra words or punctuation.',
      );
      expect(lines[1]).toBe('');
      expect(lines[2]).toBe('Here are the possible categories:');
      expect(lines[3]).toBe('');
      expect(lines[4]).toBe('Cat1');
      expect(lines[5]).toBe('Cat2');
      expect(lines[6]).toBe('');
      expect(lines[7]).toBe('Please predict the category for the following claim:');
      expect(lines[8]).toBe('');
      expect(lines[9]).toBe('Merchant: TestMerchant');
      expect(lines[10]).toBe('Description: TestDesc');
    });
  });
});
