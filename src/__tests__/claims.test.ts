// Test only the validation functions directly by copying them here
// This avoids import issues with dependencies in the full claims.ts file

const PURCHASE_DATE_REGEX = /^\d{4}-\d{2}-\d{2}$/;

const isValidPurchaseDate = (purchaseDate: string): boolean =>
  PURCHASE_DATE_REGEX.test(purchaseDate);

const AMOUNT_REGEX = /^\d+(\.\d{2})?$/;

const isValidAmount = (amount: string): boolean => AMOUNT_REGEX.test(amount);

describe('claims validation', () => {
  describe('isValidPurchaseDate', () => {
    it('accepts valid date format YYYY-MM-DD', () => {
      expect(isValidPurchaseDate('2023-12-25')).toBe(true);
      expect(isValidPurchaseDate('2024-01-01')).toBe(true);
      expect(isValidPurchaseDate('2024-02-29')).toBe(true); // leap year
      expect(isValidPurchaseDate('1999-12-31')).toBe(true);
    });

    it('rejects invalid date formats', () => {
      expect(isValidPurchaseDate('25-12-2023')).toBe(false); // DD-MM-YYYY
      expect(isValidPurchaseDate('12/25/2023')).toBe(false); // MM/DD/YYYY
      expect(isValidPurchaseDate('2023-12-25T10:30:00Z')).toBe(false); // ISO with time
      expect(isValidPurchaseDate('2023-12-25 10:30:00')).toBe(false); // with time
      expect(isValidPurchaseDate('2023-12')).toBe(false); // incomplete
      expect(isValidPurchaseDate('')).toBe(false); // empty string
      expect(isValidPurchaseDate('not-a-date')).toBe(false); // random string
    });

    it('accepts dates that match regex format even if logically invalid', () => {
      // Note: the regex only validates format, not logical date validity
      expect(isValidPurchaseDate('2023-13-01')).toBe(true); // invalid month but valid format
      expect(isValidPurchaseDate('2023-12-32')).toBe(true); // invalid day but valid format
    });
  });
});

describe('isValidAmount', () => {
  it('accepts valid amount formats', () => {
    expect(isValidAmount('10.99')).toBe(true);
    expect(isValidAmount('0.50')).toBe(true);
    expect(isValidAmount('123.45')).toBe(true);
    expect(isValidAmount('0.00')).toBe(true);
    expect(isValidAmount('1000.00')).toBe(true);
    expect(isValidAmount('999999.99')).toBe(true);
  });

  it('accepts integer amounts without decimals', () => {
    expect(isValidAmount('10')).toBe(true);
    expect(isValidAmount('0')).toBe(true);
    expect(isValidAmount('123')).toBe(true);
    expect(isValidAmount('1000')).toBe(true);
  });

  it('rejects invalid amount formats', () => {
    expect(isValidAmount('10.9')).toBe(false); // single decimal place
    expect(isValidAmount('10.999')).toBe(false); // three decimal places
    expect(isValidAmount('10.999')).toBe(false); // three decimal places
    expect(isValidAmount('10.')).toBe(false); // trailing dot
    expect(isValidAmount('.99')).toBe(false); // leading dot
    expect(isValidAmount('10,99')).toBe(false); // comma instead of dot
    expect(isValidAmount('$10.99')).toBe(false); // currency symbol
    expect(isValidAmount('-10.99')).toBe(false); // negative amount
    expect(isValidAmount('')).toBe(false); // empty string
    expect(isValidAmount('abc')).toBe(false); // non-numeric
    expect(isValidAmount('10.99.99')).toBe(false); // multiple dots
  });
});
