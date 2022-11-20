// Sets up axios to be used/provided to the app

import type { App } from 'vue';

import axios from 'axios';
import VueAxios from 'vue-axios';

export function apiAttachToApp(app: App) {
    app.use(VueAxios, axios);

    const ax_obj = app.config.globalProperties['axios'];
    ax_obj.defaults.baseURL = "https://api.runningstream.cc/api/v1/";
    if( document.location.host.includes("192.168.86.11") ) {
        ax_obj.defaults.baseURL = "http://192.168.86.11:3031/api/v1/";
    }
    ax_obj.defaults.withCredentials = true;

    // Changes requests to use url form encoding instead of JSON, thus
    // avoiding a preflight
    ax_obj.defaults.transformRequest = [(data: string, _headers: any) => {
        return new URLSearchParams(data);
    }];

    return app.provide('axios', app.config.globalProperties['axios']);
}
