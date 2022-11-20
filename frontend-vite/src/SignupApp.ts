import { createApp } from 'vue';
import './style.css';
import SignupApp from './SignupApp.vue';

import { apiAttachToApp } from './api_js/attachAPI.js';

apiAttachToApp(createApp(SignupApp)).mount('#signupApp');
