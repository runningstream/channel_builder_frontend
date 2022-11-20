import { createApp } from 'vue';
import './style.css';
import LoginApp from './LoginApp.vue';

import { apiAttachToApp } from './api_js/attachAPI.js';

apiAttachToApp(createApp(LoginApp)).mount('#loginApp');
