import { createApp } from 'vue';
import './style.css';
import EditorApp from './EditorApp.vue';

import { apiAttachToApp } from './api_js/attachAPI.js';

apiAttachToApp(createApp(EditorApp)).mount('#editorApp');
