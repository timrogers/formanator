import { maybeGetAccessToken } from '../config.js';

// Mock the filesystem operations to avoid actual file system interactions
jest.mock('fs', () => ({
  readFileSync: jest.fn(),
  writeFileSync: jest.fn(),
  existsSync: jest.fn(),
}));

jest.mock('os', () => ({
  homedir: () => '/mock/home',
}));

describe('config', () => {
  describe('maybeGetAccessToken', () => {
    it('returns the provided token if truthy', () => {
      expect(maybeGetAccessToken('my-token')).toBe('my-token');
      expect(maybeGetAccessToken('another-token')).toBe('another-token');
    });

    it('calls getAccessToken when token is null', () => {
      // Since getAccessToken depends on file system, we'll mock its behavior
      // eslint-disable-next-line @typescript-eslint/no-require-imports
      const fs = require('fs');
      fs.existsSync.mockReturnValue(false);

      expect(maybeGetAccessToken(null)).toBe(null);
    });

    it('calls getAccessToken when token is undefined', () => {
      // eslint-disable-next-line @typescript-eslint/no-require-imports
      const fs = require('fs');
      fs.existsSync.mockReturnValue(false);

      expect(maybeGetAccessToken(undefined)).toBe(null);
    });

    it('calls getAccessToken when token is empty string', () => {
      // eslint-disable-next-line @typescript-eslint/no-require-imports
      const fs = require('fs');
      fs.existsSync.mockReturnValue(false);

      expect(maybeGetAccessToken('')).toBe(null);
    });
  });
});
