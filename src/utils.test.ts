import { beforeEach, describe, expect, it, jest } from '@jest/globals';

// Mock dependencies before importing
jest.mock('chalk');
jest.mock('prompt-sync');

import { serializeError, actionRunner } from '../src/utils.js';

describe('utils', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  describe('serializeError', () => {
    it('should return the error message when given an Error object', () => {
      const error = new Error('Test error message');
      const result = serializeError(error);
      expect(result).toBe('Test error message');
    });

    it('should return the string when given a string', () => {
      const errorString = 'Simple error string';
      const result = serializeError(errorString);
      expect(result).toBe(errorString);
    });

    it('should return JSON string when given an object', () => {
      const errorObject = { code: 500, message: 'Server error' };
      const result = serializeError(errorObject);
      expect(result).toBe(JSON.stringify(errorObject));
    });

    it('should return JSON string when given a number', () => {
      const errorNumber = 404;
      const result = serializeError(errorNumber);
      expect(result).toBe(JSON.stringify(errorNumber));
    });

    it('should handle null and undefined', () => {
      expect(serializeError(null)).toBe('null');
      expect(serializeError(undefined)).toBe(undefined);
    });
  });

  describe('actionRunner', () => {
    it('should call the function and return its result when successful', async () => {
      const mockFn = jest.fn<(...args: any[]) => Promise<void>>().mockResolvedValue();
      const wrappedFn = actionRunner(mockFn);

      await wrappedFn('arg1', 'arg2');

      expect(mockFn).toHaveBeenCalledWith('arg1', 'arg2');
      expect(mockFn).toHaveBeenCalledTimes(1);
    });

    it('should catch errors and call error handler when function throws', async () => {
      const testError = new Error('Test error');
      const mockFn = jest.fn<(...args: any[]) => Promise<void>>().mockRejectedValue(testError);
      const wrappedFn = actionRunner(mockFn);

      // Mock process.exit to prevent test from actually exiting
      const mockExit = jest.spyOn(process, 'exit').mockImplementation(() => undefined as never);
      const mockConsoleError = jest.spyOn(console, 'error').mockImplementation(() => {});

      await wrappedFn('arg1', 'arg2');

      expect(mockFn).toHaveBeenCalledWith('arg1', 'arg2');
      expect(mockConsoleError).toHaveBeenCalledWith(expect.stringContaining('Test error'));
      expect(mockExit).toHaveBeenCalledWith(1);

      mockExit.mockRestore();
      mockConsoleError.mockRestore();
    });

    it('should handle non-Error thrown values', async () => {
      const mockFn = jest.fn<(...args: any[]) => Promise<void>>().mockRejectedValue('String error');
      const wrappedFn = actionRunner(mockFn);

      const mockExit = jest.spyOn(process, 'exit').mockImplementation(() => undefined as never);
      const mockConsoleError = jest.spyOn(console, 'error').mockImplementation(() => {});

      await wrappedFn();

      // When a non-Error is thrown, error.message is undefined, so chalk.red(undefined) is called
      expect(mockConsoleError).toHaveBeenCalledWith('red:undefined');
      expect(mockExit).toHaveBeenCalledWith(1);

      mockExit.mockRestore();
      mockConsoleError.mockRestore();
    });
  });
});