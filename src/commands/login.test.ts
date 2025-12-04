import { beforeEach, describe, expect, it, jest } from '@jest/globals';

// Mock dependencies
jest.mock('open', () => jest.fn());
jest.mock('chalk', () => ({
  green: (msg: string) => msg,
  yellow: (msg: string) => msg,
}));
jest.mock('../utils.js', () => ({
  actionRunner: (fn: any) => fn,
  prompt: jest.fn(),
}));
jest.mock('../config.js', () => ({
  storeConfig: jest.fn(),
  getEmail: jest.fn(),
}));
jest.mock('../forma.js', () => ({
  exchangeIdAndTkForAccessToken: jest.fn(),
  requestMagicLink: jest.fn(),
}));

import open from 'open';
import { prompt } from '../utils.js';
import { storeConfig } from '../config.js';
import { exchangeIdAndTkForAccessToken } from '../forma.js';
import loginCommand from './login.js';

describe('login command', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('should open browser and prompt for magic link', async () => {
    const mockOpen = open as unknown as jest.Mock;
    const mockPrompt = prompt as unknown as jest.Mock;
    const mockExchange = exchangeIdAndTkForAccessToken as jest.MockedFunction<typeof exchangeIdAndTkForAccessToken>;
    const mockStoreConfig = storeConfig as unknown as jest.Mock;

    // Mock prompt to return a valid magic link
    // URL encoded: https://client.joinforma.com/login/magic?id=test-id&tk=test-tk
    // link param: https%3A%2F%2Fclient.joinforma.com%2Flogin%2Fmagic%3Fid%3Dtest-id%26tk%3Dtest-tk
    mockPrompt.mockReturnValue('https://joinforma.page.link/?link=https%3A%2F%2Fclient.joinforma.com%2Flogin%2Fmagic%3Fid%3Dtest-id%26tk%3Dtest-tk');
    
    // Mock exchange to return token
    mockExchange.mockResolvedValue('test-token');

    // Execute the command
    await loginCommand.parseAsync(['node', 'test']);

    // Verify open was called
    expect(mockOpen).toHaveBeenCalledWith('https://client.joinforma.com/login?type=magic');

    // Verify prompt was called
    expect(mockPrompt).toHaveBeenCalled();

    // Verify exchange was called
    expect(mockExchange).toHaveBeenCalledWith('test-id', 'test-tk');

    // Verify config was stored
    expect(mockStoreConfig).toHaveBeenCalledWith({ accessToken: 'test-token' });
  });
});
