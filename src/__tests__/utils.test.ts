import { serializeError, validateAxiosStatus, checkFor403Error } from '../utils.js';

// Mock chalk to avoid ESM issues in tests
jest.mock('chalk', () => ({
  default: {
    red: (msg: string) => msg,
  },
}));

describe('utils', () => {
  describe('serializeError', () => {
    it('returns string as-is', () => {
      expect(serializeError('test error')).toBe('test error');
    });

    it('returns Error message', () => {
      const error = new Error('test error message');
      expect(serializeError(error)).toBe('test error message');
    });

    it('returns JSON.stringify for other types', () => {
      const obj = { message: 'test' };
      expect(serializeError(obj)).toBe('{"message":"test"}');
    });

    it('handles null', () => {
      expect(serializeError(null)).toBe('null');
    });

    it('handles undefined', () => {
      expect(serializeError(undefined)).toBe(undefined);
    });

    it('handles numbers', () => {
      expect(serializeError(404)).toBe('404');
    });
  });

  describe('validateAxiosStatus', () => {
    it('always returns true', () => {
      expect(validateAxiosStatus()).toBe(true);
    });
  });

  describe('checkFor403Error', () => {
    it('throws error for 403 status', () => {
      expect(() => checkFor403Error(403)).toThrow(
        'Your Forma token has expired. Please log in again with `formanator login`.',
      );
    });

    it('does not throw for other status codes', () => {
      expect(() => checkFor403Error(200)).not.toThrow();
      expect(() => checkFor403Error(404)).not.toThrow();
      expect(() => checkFor403Error(500)).not.toThrow();
    });
  });
});
