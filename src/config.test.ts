import { beforeEach, describe, expect, it, jest } from '@jest/globals';

// Mock modules before importing
jest.mock('fs');
jest.mock('os');

import fs from 'fs';
import os from 'os';
import { getAccessToken, setAccessToken, maybeGetAccessToken } from '../src/config.js';

const mockFs = fs as jest.Mocked<typeof fs>;
const mockOs = os as jest.Mocked<typeof os>;

describe('config', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    mockOs.homedir.mockReturnValue('/mock/home');
  });

  describe('getAccessToken', () => {
    it('should return null when config file does not exist', () => {
      mockFs.existsSync.mockReturnValue(false);

      const result = getAccessToken();

      expect(result).toBeNull();
      expect(mockFs.existsSync).toHaveBeenCalledWith('/mock/home/.formanatorrc.json');
    });

    it('should return access token when config file exists and is valid', () => {
      mockFs.existsSync.mockReturnValue(true);
      mockFs.readFileSync.mockReturnValue('{"accessToken":"test-token-123"}');

      const result = getAccessToken();

      expect(result).toBe('test-token-123');
      expect(mockFs.readFileSync).toHaveBeenCalledWith('/mock/home/.formanatorrc.json', {
        encoding: 'utf-8',
      });
    });

    it('should handle JSON parsing errors gracefully', () => {
      mockFs.existsSync.mockReturnValue(true);
      mockFs.readFileSync.mockReturnValue('invalid json');

      expect(() => getAccessToken()).toThrow();
    });
  });

  describe('setAccessToken', () => {
    it('should write access token to config file', () => {
      const testToken = 'new-test-token-456';

      setAccessToken(testToken);

      expect(mockFs.writeFileSync).toHaveBeenCalledWith(
        '/mock/home/.formanatorrc.json',
        '{"accessToken":"new-test-token-456"}',
      );
    });
  });

  describe('maybeGetAccessToken', () => {
    it('should return provided token when token is provided', () => {
      const providedToken = 'provided-token';

      const result = maybeGetAccessToken(providedToken);

      expect(result).toBe(providedToken);
      // Should not call getAccessToken
      expect(mockFs.existsSync).not.toHaveBeenCalled();
    });

    it('should return null when provided token is null and no config exists', () => {
      mockFs.existsSync.mockReturnValue(false);

      const result = maybeGetAccessToken(null);

      expect(result).toBeNull();
      expect(mockFs.existsSync).toHaveBeenCalledWith('/mock/home/.formanatorrc.json');
    });

    it('should return null when provided token is undefined and no config exists', () => {
      mockFs.existsSync.mockReturnValue(false);

      const result = maybeGetAccessToken(undefined);

      expect(result).toBeNull();
      expect(mockFs.existsSync).toHaveBeenCalledWith('/mock/home/.formanatorrc.json');
    });

    it('should return config token when provided token is null but config exists', () => {
      mockFs.existsSync.mockReturnValue(true);
      mockFs.readFileSync.mockReturnValue('{"accessToken":"config-token"}');

      const result = maybeGetAccessToken(null);

      expect(result).toBe('config-token');
    });

    it('should return config token when provided token is empty string but config exists', () => {
      mockFs.existsSync.mockReturnValue(true);
      mockFs.readFileSync.mockReturnValue('{"accessToken":"config-token"}');

      const result = maybeGetAccessToken('');

      expect(result).toBe('config-token');
    });
  });
});
