import os from 'os';
import path from 'path';
import { readFileSync, writeFileSync, existsSync } from 'fs';

const CONFIG_FILENAME = '.formanatorrc.json';
const CONFIG_PATH = path.join(os.homedir(), CONFIG_FILENAME);

interface Config {
  accessToken: string;
  email?: string;
}

export const maybeGetAccessToken = (
  maybeAccessToken: string | null | undefined,
): string | null => {
  if (maybeAccessToken) return maybeAccessToken;
  return getAccessToken();
};

export const getAccessToken = (): string | null => {
  if (!existsSync(CONFIG_PATH)) {
    return null;
  }

  const rawConfig = readFileSync(CONFIG_PATH, { encoding: 'utf-8' });
  const parsedConfig = JSON.parse(rawConfig) as Config;

  return parsedConfig.accessToken;
};

export const getEmail = (): string | null => {
  if (!existsSync(CONFIG_PATH)) {
    return null;
  }

  const rawConfig = readFileSync(CONFIG_PATH, { encoding: 'utf-8' });
  const parsedConfig = JSON.parse(rawConfig) as Config;

  return parsedConfig.email || null;
};

export const setAccessToken = (accessToken: string, email?: string): void => {
  let config: Config = { accessToken };
  
  if (existsSync(CONFIG_PATH)) {
    const rawConfig = readFileSync(CONFIG_PATH, { encoding: 'utf-8' });
    const existingConfig = JSON.parse(rawConfig) as Config;
    config = { ...existingConfig, accessToken };
  }
  
  if (email) {
    config.email = email;
  }
  
  writeFileSync(CONFIG_PATH, JSON.stringify(config));
};
