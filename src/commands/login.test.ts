import { beforeEach, describe, expect, it, jest } from '@jest/globals';

jest.mock('open');
jest.mock('../utils.js', () => {
  return {
    prompt: jest.fn(),
    actionRunner: (fn: any) => fn,
    serializeError: (e: any) => JSON.stringify(e),
  };
});
jest.mock('../config.js');
jest.mock('../forma.js');
jest.mock('chalk', () => ({
  green: (msg: string) => msg,
  yellow: (msg: string) => msg,
  red: (msg: string) => msg,
}));

import open from 'open';
import { prompt } from '../utils.js';
import { storeConfig } from '../config.js';
import { exchangeIdAndTkForAccessToken } from '../forma.js';
import loginCommand from './login.js';

const mockOpen = open as unknown as jest.MockedFunction<typeof open>;
const mockPrompt = prompt as jest.MockedFunction<typeof prompt>;
const mockStoreConfig = storeConfig as jest.MockedFunction<typeof storeConfig>;
const mockExchangeIdAndTkForAccessToken = exchangeIdAndTkForAccessToken as jest.MockedFunction<typeof exchangeIdAndTkForAccessToken>;

describe('login command', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    jest.spyOn(console, 'log').mockImplementation(() => {});
  });

  it('should open browser and prompt for magic link', async () => {
    const magicLink = 'https://joinforma.page.link/?link=https%3A%2F%2Fclient.joinforma.com%2Flogin%2Fmagic%3Fid%3Dtest-id%26tk%3Dtest-tk';
    mockPrompt.mockReturnValue(magicLink);
    mockExchangeIdAndTkForAccessToken.mockResolvedValue('test-access-token');

    await loginCommand.parseAsync(['node', 'test']);

    expect(mockOpen).toHaveBeenCalledWith('https://client.joinforma.com/login?type=magic');
    expect(mockPrompt).toHaveBeenCalled();
    expect(mockExchangeIdAndTkForAccessToken).toHaveBeenCalledWith('test-id', 'test-tk');
    expect(mockStoreConfig).toHaveBeenCalledWith({ accessToken: 'test-access-token' });
  });
});
