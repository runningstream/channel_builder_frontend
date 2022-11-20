import { createApp } from 'vue';
import './style.css';
import ValidationApp from './ValidationApp.vue';

import { apiAttachToApp } from './api_js/attachAPI.js';

apiAttachToApp(createApp(ValidationApp)).mount('#validationApp');
