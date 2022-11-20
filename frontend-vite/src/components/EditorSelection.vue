<script setup lang="ts">
    import { ref, onMounted } from "vue";

    import type { Ref } from "vue";
    import type { VideoType } from "../api_js/serverAPI";

    import { apiGetChannelList, apiGetChannelLists, apiSetChannelList,
        apiSetActiveChannel, apiCreateChannelList, apiGetActiveChannelName,
        apiDeleteChannel, apiRenameChannel
    } from "../api_js/serverAPI";

    import VideoListEntry from "./VideoListEntry.vue";
    import ChannelList from "./ChannelList.vue";
    import NewChannel from "./NewChannel.vue";
    import Alert from "./Alert.vue";
    import Spinner from "./Spinner.vue";

    const channel_list_as_entry: Ref<VideoType> = ref({});
    const channel_list_list: Ref<Array<string>> = ref([]);
    //const video_selected: Ref<VideoType> = ref({});
    const active_channel: Ref<string | undefined> = ref(undefined);
    const channel_selected: Ref<string | undefined> = ref(undefined);
    const show_new_channel: Ref<boolean> = ref(false);
    const show_spinner: Ref<boolean> = ref(false);
    const alert_api_error_show: Ref<boolean> = ref(false);
    const alert_api_error_text: Ref<string> = ref("");

    onMounted( get_channel_lists );

    function select_video( _video: VideoType ) : void {
        // Nothing to do right now
    }

    function select_channel( channel : string | undefined ) : void {
        show_spinner.value = true;

        channel_selected.value = channel;
        if( channel == undefined ) {
            channel_list_as_entry.value = {};
            show_spinner.value = false;
            return;
        }
        apiGetChannelList( channel )
            .then( (chan_list) => {
                channel_list_as_entry.value.name = channel;
                channel_list_as_entry.value.type = "toplevel";
                channel_list_as_entry.value.entries = chan_list.data.entries;
                show_spinner.value = false;
            } )
            .catch( (error: any) => {
                show_spinner.value = false;
                alert_api_error_text.value = `Error retrieving channel - consider refreshing: ${error}`;
                alert_api_error_show.value = true;
            } );
    }

    function get_channel_lists() : void {
        select_channel( undefined );
        active_channel.value = undefined;

        // The spinner will get unset when select_channel completes, or there's an error
        // All roads lead to select_channel, or an error
        show_spinner.value = true;

        apiGetChannelLists()
            .then( (ret_chan_list_list) => {
                channel_list_list.value = ret_chan_list_list.data;

                apiGetActiveChannelName()
                    .then( (active_channel_name) => {
                        active_channel.value = active_channel_name.data;
                        select_channel(active_channel_name.data);
                    })
                    .catch( (error: any) => {
                        show_spinner.value = false;
                        alert_api_error_text.value = `Error getting active channel name - consider refreshing: ${error}`;
                        alert_api_error_show.value = true;
                    });
            })
            .catch( (error: any) => {
                show_spinner.value = false;
                alert_api_error_text.value = `Error getting channel lists - consider refreshing: ${error}`;
                alert_api_error_show.value = true;
            });
    }

    function update_entry( new_entry: VideoType ) : void {
        channel_list_as_entry.value = new_entry;

        if( channel_list_as_entry.value.name == undefined || 
                channel_list_as_entry.value.entries == undefined )
        {
            alert_api_error_text.value = "Unknown error - portions of channel list are undefined - consider refreshing.";
            alert_api_error_show.value = true;
            return;
        }

        apiSetChannelList(channel_list_as_entry.value.name, channel_list_as_entry.value.entries)
                .catch( (error: any) => { 
                    alert_api_error_text.value = `Error updating channel - consider refreshing: ${error}`;
                    alert_api_error_show.value = true;
                });
    }

    function set_active_channel(name: string | undefined) : void {
        if( name != undefined ) {
            apiSetActiveChannel(name)
                .then( () => { active_channel.value = name; } )
                .catch( (error: any) => { 
                    alert_api_error_text.value = `Error updating active channel - consider refreshing: ${error}`;
                    alert_api_error_show.value = true;
                });
        }
    }

    function new_channel(name: string) : void {
        // Check for a duplicate channel
        for( const chan_name of channel_list_list.value ) {
            if( chan_name == name ) {
                alert_api_error_text.value = "You cannot create two channels with the same name.";
                alert_api_error_show.value = true;
                return;
            }
        }

        // The spinner will be unset by get_channel_lists or on error
        show_spinner.value = true;

        apiCreateChannelList(name)
            .then(get_channel_lists)
            .catch( (error: any) => {
                show_spinner.value = false;
                alert_api_error_text.value = `Error creating channel - consider refreshing: ${error}`;
                alert_api_error_show.value = true;
            });
    }

    function rename_channel(cur_name: string | undefined, new_name: string) : void {
        if( cur_name == undefined ) { return; }

        // Check for a duplicate channel
        for( const chan_name of channel_list_list.value ) {
            if( chan_name == new_name ) {
                alert_api_error_text.value = "You cannot create two channels with the same name.";
                alert_api_error_show.value = true;
                return;
            }
        }

        // The spinner will be unset by get_channel_lists or on error
        show_spinner.value = true;

        apiRenameChannel(cur_name, new_name)
            .then(get_channel_lists)
            .catch( (error: any) => {
                show_spinner.value = false;
                alert_api_error_text.value = `Error renaming channel - consider refreshing: ${error}`;
                alert_api_error_show.value = true;
            });
    }

    function delete_channel(name: string | undefined) : void {
        if( name == undefined ) { return; }

        // The spinner will be unset by get_channel_lists or on error
        show_spinner.value = true;

        apiDeleteChannel(name)
            .then(get_channel_lists)
            .catch( (error: any) => {
                show_spinner.value = false;
                alert_api_error_text.value = `Error deleting channel - consider refreshing: ${error}`;
                alert_api_error_show.value = true;
            });
    }
</script>

<template>
    <div id="content">
        <NewChannel :display="show_new_channel"
            @cancelModal="show_new_channel=false"
            @saveModal="show_new_channel=false; new_channel($event);"
        />
        <Alert :text="alert_api_error_text"
            :display="alert_api_error_show" @closeModal="alert_api_error_show=false"
        />
        <Spinner :display="show_spinner" />
        <div id="channel_mng">
            <div class="buttonarea">
                <input type="button" value="New Channel" @click="show_new_channel=true">
            </div>
            <ChannelList :channelList="channel_list_list" :channelSelected="channel_selected"
                :activeChannel="active_channel" @channelSelected="select_channel"
            />
        </div>
        <VideoListEntry id="video_list" :entry="channel_list_as_entry"
            @videoSelected="select_video"
            @updateEntry="update_entry"
            @renameChannel="rename_channel"
            @deleteChannel="delete_channel"
            @setActiveChannel="set_active_channel"
        />
    </div>
</template>

<style scoped>
    #content {
        display: flex;
        flex-direction: row-reverse;
        border-width: 1px;
        border-radius: 20px;
        padding: 8px;
    }

    #channel_mng {
        width: 20%;
    }

    #video_list {
        width: 80%;
        text-align: center;
    }

    .buttonarea {
        display: flex;
        flex-wrap: wrap;
        flex: 1 0 50px;
        justify-content: center;
    }
    .buttonarea input { 
        margin-bottom: auto;
    }

    @media (max-width: 700px) {
        #content {
            display: block;
        }
        #channel_mng {
            width: 100%;
        }
        #video_list {
            width: 100%;
            border-top: 1px var(--color-scarlet) solid;
            border-radius: 0;
            margin-top: 10px;
            padding-top: 10px;
        }
    }

    #video_list {
        margin-bottom: 5px;
    }
</style>
