export interface OAuthConfig {
    clientId: string;
    clientSecret: string;
    redirectUri: string;
    scopes: string[];
    authUrl: string;
    tokenUrl: string;
}
export declare const oauthConfigs: {
    readonly google: {
        readonly clientId: string;
        readonly clientSecret: string;
        readonly redirectUri: string;
        readonly scopes: readonly ["https://www.googleapis.com/auth/calendar.readonly", "https://www.googleapis.com/auth/gmail.readonly", "https://www.googleapis.com/auth/drive.readonly"];
        readonly authUrl: "https://accounts.google.com/o/oauth2/v2/auth";
        readonly tokenUrl: "https://oauth2.googleapis.com/token";
    };
    readonly notion: {
        readonly clientId: string;
        readonly clientSecret: string;
        readonly redirectUri: string;
        readonly scopes: readonly [];
        readonly authUrl: "https://api.notion.com/v1/oauth/authorize";
        readonly tokenUrl: "https://api.notion.com/v1/oauth/token";
    };
    readonly microsoft: {
        readonly clientId: string;
        readonly clientSecret: string;
        readonly redirectUri: string;
        readonly scopes: readonly ["https://graph.microsoft.com/calendars.read", "https://graph.microsoft.com/mail.read", "https://graph.microsoft.com/files.read"];
        readonly authUrl: "https://login.microsoftonline.com/common/oauth2/v2.0/authorize";
        readonly tokenUrl: "https://login.microsoftonline.com/common/oauth2/v2.0/token";
    };
    readonly github: {
        readonly clientId: string;
        readonly clientSecret: string;
        readonly redirectUri: string;
        readonly scopes: readonly ["repo", "user:email"];
        readonly authUrl: "https://github.com/login/oauth/authorize";
        readonly tokenUrl: "https://github.com/login/oauth/access_token";
    };
    readonly strava: {
        readonly clientId: string;
        readonly clientSecret: string;
        readonly redirectUri: string;
        readonly scopes: readonly ["read,activity:read_all"];
        readonly authUrl: "https://www.strava.com/oauth/authorize";
        readonly tokenUrl: "https://www.strava.com/oauth/token";
    };
};
export type OAuthProvider = keyof typeof oauthConfigs;
//# sourceMappingURL=oauth-apps.d.ts.map