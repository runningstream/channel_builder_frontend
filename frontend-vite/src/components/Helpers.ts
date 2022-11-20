import { after_login_page } from '../CB_Config';

export function jump_to_after_login() {
    jump_to_page(after_login_page);
}

export function jump_to_login() {
    jump_to_page("./index.html");
}

function jump_to_page(url: string) {
    console.log("Jumping TS");
    window.location.href = url;
}

export function notify_error(str: string) {
    console.error(str);
}
