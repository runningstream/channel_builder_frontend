<script setup lang="ts">
    import VideoListEntry from "./VideoListEntry.vue";

    import type { VideoType } from "../api_js/serverAPI";

    const params = defineProps({
        channelList: Object as () => Array<VideoType>,
        depth: {
            type: Number,
            default: 0,
        },
        editEnable: Boolean,
    });
    const emit = defineEmits<{
        (e: "videoSelected", entry: VideoType) : void
        (e: "updateEntries", entries: Array<VideoType>) : void
    }>();

    /* An entry has requested it be deleted
    */
    function delete_entry( delEntry: VideoType ) : void {
        if( params.channelList == undefined ) {
            console.error("channelList was undefined");
            return;
        }
        const new_chan_list = params.channelList.filter((entry: VideoType) => { return entry != delEntry; });
        emit("updateEntries", new_chan_list);
    }

    /* An entry has requested it be moved up
    */
    function move_up_entry( upEntry: VideoType ) : void {
        if( params.channelList == undefined ) {
            console.error("ChannelList undefined!");
            return;
        }

        let made_change = false;
        const cur = params.channelList;
        const new_list = cur.map((val: VideoType, ind: number) => { 
            if( ind < cur.length - 1 && cur[ind+1] == upEntry ) {
                made_change = true;
                return upEntry;
            } else if( ind > 0 && cur[ind] == upEntry ) {
                made_change = true;
                return cur[ind-1] as VideoType;
            } else {
                return val;
            }
        });
        if( !made_change ) {
            console.error("No changes!");
            return;
        }
        emit("updateEntries", new_list);
    }

    /* An entry has requested it be moved down
    */
    function move_down_entry( downEntry: VideoType ) : void {
        if( params.channelList == undefined ) {
            console.error("ChannelList undefined!");
            return;
        }

        let made_change = false;
        const cur = params.channelList;
        const new_list = cur.map((val: VideoType, ind: number) => { 
            if( ind < (cur.length - 1) && cur[ind] == downEntry ) {
                made_change = true;
                return cur[ind + 1] as VideoType;
            } else if( ind != 0 && cur[ind-1] == downEntry ) {
                made_change = true;
                return downEntry;
            } else {
                return val;
            }
        });
        if( !made_change ) {
            console.error("No changes!");
            return;
        }
        emit("updateEntries", new_list);
    }

    /* An entry has requested it be updated
    */
    function update_entry( new_entry: VideoType, cur_entry: VideoType ) : void {
        if( params.channelList == undefined ) {
            console.error("channelList was undefined");
            return;
        }

        const new_entries_list = params.channelList.map((entry: VideoType) => { return entry == cur_entry ? new_entry : entry; });
        emit("updateEntries", new_entries_list);
    }
</script>

<template>
    <div class="video_list" :class="'vidListDepth' + depth">
        <VideoListEntry class="video_list_entry"
            v-if="channelList != undefined"
            v-for="(entry, index) in channelList" :entry="entry"
            :editEnable="editEnable"
            :depth="(depth + 1) % 2"
            :firstEntry="index == 0 ? true : false"
            :lastEntry="index == channelList.length - 1 ? true : false"
            @videoSelected="$emit('videoSelected', $event)"
            @deleteEntry="delete_entry(entry)"
            @moveUpEntry="move_up_entry(entry)"
            @moveDownEntry="move_down_entry(entry)"
            @updateEntry="update_entry($event, entry)"
        />
    </div>
</template>

<style scoped>
    .video_list.vidListDepth1 {
        background: var(--color-ltolive);
    }
    .video_list.vidListDepth0 {
        background: var(--color-teal);
    }
    .video_list {
        padding: 5px;
        margin-top: 5px;
        position: relative;
        border-radius: 10px;
    }
    .video_list .video_list {
        padding: 5px 2px 5px 10px;
    }
</style>
