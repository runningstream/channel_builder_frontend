// Must be attached to the app using attachAPI.js
// After importing this, but still during setup and before running an API func
// run apiInitialize.  Perhaps in a base page that gets used everywhere

import { api_suffix } from '../CB_Config';

import { inject } from "vue";

export interface VideoType {
    name?: string,
    type?: string, //video, audio, sublist
    image?: string,
    videourl?: string,
    loop?: boolean,
    videotype?: string,
    entries?: Array<VideoType>,
}
export function newVideoTypeMedia() : VideoType {
    let output: VideoType = {};
    output.name = "";
    output.type = "video";
    output.image = "";
    output.videourl = "";
    output.loop = false;
    output.videotype = "mp4";
    return output;
}
export function newVideoTypeSublist() : VideoType {
    let output: VideoType = {};
    output.name = "";
    output.type = "sublist";
    output.image = "";
    output.entries = [];
    return output;
}

export interface ChannelData {
    data: { entries: Array<VideoType>, },
}
export interface ChannelListData {
    data: Array<string>,
}
export interface ChannelListName {
    data: string,
}

let axios: any = undefined;

export function apiInitialize() : void{
    axios = inject("axios");
}

export function apiValidateSession() : Promise<void> {
    validateInitialized();
    return axios.get(`validate_session_${api_suffix}`);
}


export function apiAuthenticate(username: string, password: string) : Promise<void> {
    validateInitialized();
    const data = {
        "username": username,
        "password": password,
    };
    return axios.post(`authenticate_${api_suffix}`, data);
}

export function apiCreateAccount(username: string, password: string) : Promise<void> {
    validateInitialized();
    const data = {
        "username": username,
        "password": password,
    };
    return axios.post(`create_account`, data);
}

export function apiValidateAccount(val_code: string) : Promise<void> {
    validateInitialized();
    return axios.get(`validate_account?val_code=${val_code}`);
}

export function apiLogout() : Promise<void> {
    validateInitialized();
    return axios.get(`logout_session_${api_suffix}`);
}

export function apiGetChannelLists() : Promise<ChannelListData> {
    validateInitialized();
    return axios.get(`get_channel_lists_${api_suffix}`);
}

export function apiGetActiveChannel() : Promise<ChannelData> {
    validateInitialized();
    return axios.get(`get_active_channel_${api_suffix}`);
}

export function apiGetActiveChannelName() : Promise<ChannelListName> {
    validateInitialized();
    return axios.get(`get_active_channel_name_${api_suffix}`);
}

export function apiGetChannelList(list: string) : Promise<ChannelData> {
    validateInitialized();
    return axios.get(`get_channel_list_${api_suffix}?list_name=${list}`);
}

export function apiDeleteChannel(list: string) : Promise<void> {
    validateInitialized();
    return axios.delete(`delete_channel_${api_suffix}?listname=${list}`);
}

export function apiRenameChannel(listname: string, newlistname: string) : Promise<void> {
    const data = {
        "listname": listname,
        "newlistname": newlistname,
    };
    validateInitialized();
    return axios.post(`rename_channel_${api_suffix}`, data);
}

export function apiSetChannelList(listname: string, listdata: Array<VideoType>) : Promise<void> {
    const data = {
        "listname": listname,
        "listdata": JSON.stringify({"entries": listdata}),
    };
    validateInitialized();
    return axios.post(`set_channel_list_${api_suffix}`, data);
}

export function apiSetActiveChannel(listname: string) : Promise<void> {
    const data = {
        "listname": listname,
    };
    validateInitialized();
    return axios.post(`set_active_channel_${api_suffix}`, data);
}

export function apiCreateChannelList(listname: string) : Promise<void> {
    const data = {
        "listname": listname,
    };
    validateInitialized();
    return axios.post(`create_channel_list_${api_suffix}`, data);
}

function validateInitialized() : void {
    if ( axios === undefined ) {
        console.error("axios is undefined - run apiInitialize in component setup");
    }
}
