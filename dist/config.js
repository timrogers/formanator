import os from "os";
import path from "path";
import { readFileSync, writeFileSync, existsSync } from "fs";
const CONFIG_FILENAME = ".formanatorrc.json";
const CONFIG_PATH = path.join(os.homedir(), CONFIG_FILENAME);
export const maybeGetAccessToken = (maybeAccessToken) => {
    if (maybeAccessToken)
        return maybeAccessToken;
    return getAccessToken();
};
export const getAccessToken = () => {
    if (!existsSync(CONFIG_PATH)) {
        return null;
    }
    const rawConfig = readFileSync(CONFIG_PATH, { encoding: "utf-8" });
    const parsedConfig = JSON.parse(rawConfig);
    return parsedConfig.accessToken;
};
export const setAccessToken = (accessToken) => {
    const config = { accessToken };
    writeFileSync(CONFIG_PATH, JSON.stringify(config));
};
