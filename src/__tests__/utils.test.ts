import { serializeError } from '../utils.js';

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
});
