import { beforeEach, describe, expect, it, jest } from '@jest/globals';

jest.mock('open');
jest.mock('../config.js');
jest.mock('../forma.js');
jest.mock('../version.js', () => ({ default: '0.0.0-test' }));

import open from 'open';
import { storeConfig } from '../config.js';
import { exchangeIdAndTkForAccessToken } from '../forma.js';
import { prompt } from '../utils.js';
import command from './login.js';

const mockOpen = open as jest.MockedFunction<typeof open>;
const mockStoreConfig = storeConfig as jest.MockedFunction<typeof storeConfig>;
const mockExchange = exchangeIdAndTkForAccessToken as jest.MockedFunction<
  typeof exchangeIdAndTkForAccessToken
>;
const mockPrompt = prompt as jest.MockedFunction<typeof prompt>;

const VALID_MAGIC_LINK =
  'https://joinforma.page.link/?link=https%3A%2F%2Fclient.joinforma.com%2Fauth%2Fmagic%3Fid%3Dtest-id-123%26tk%3Dtest-tk-456';

describe('login command', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    mockExchange.mockResolvedValue('test-access-token');
  });

  describe('with --magic-link option', () => {
    it('should parse the magic link, exchange for token, and store config', async () => {
      await command.parseAsync(['node', 'test', '--magic-link', VALID_MAGIC_LINK]);

      expect(mockExchange).toHaveBeenCalledWith('test-id-123', 'test-tk-456');
      expect(mockStoreConfig).toHaveBeenCalledWith({ accessToken: 'test-access-token' });
    });

    it('should not open the browser', async () => {
      await command.parseAsync(['node', 'test', '--magic-link', VALID_MAGIC_LINK]);

      expect(mockOpen).not.toHaveBeenCalled();
    });

    it('should not prompt the user', async () => {
      await command.parseAsync(['node', 'test', '--magic-link', VALID_MAGIC_LINK]);

      expect(mockPrompt).not.toHaveBeenCalled();
    });

    it('should error for an invalid magic link URL', async () => {
      await command.parseAsync([
        'node',
        'test',
        '--magic-link',
        'https://example.com/not-a-magic-link',
      ]);

      expect(mockExchange).not.toHaveBeenCalled();
      expect(mockStoreConfig).not.toHaveBeenCalled();
    });
  });

  describe('without --magic-link option', () => {
    it('should open the browser and prompt for a magic link', async () => {
      mockPrompt.mockReturnValueOnce('').mockReturnValueOnce(VALID_MAGIC_LINK);

      await command.parseAsync(['node', 'test']);

      expect(mockOpen).toHaveBeenCalledWith(
        'https://client.joinforma.com/login?type=magic',
      );
      expect(mockExchange).toHaveBeenCalledWith('test-id-123', 'test-tk-456');
      expect(mockStoreConfig).toHaveBeenCalledWith({ accessToken: 'test-access-token' });
    });
  });
});
