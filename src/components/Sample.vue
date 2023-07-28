<template>
    <a-tooltip :open="tooltipOpen" :style="{
        position: 'absolute',
    }" :align="{
    offset: [tooltipLeft, tooltipTop]
}">
        <template #title>
            <p>{{ tooltipTitle }}</p>
            <p>{{ tooltipContent }}</p>
        </template>
        <!--        <div id="tooltip" :style="{-->
        <!--            position: 'absolute',-->
        <!--            left: tooltipLeft + 'px',-->
        <!--            top: tooltipTop + 'px',-->
        <!--        }"></div>-->
    </a-tooltip>
    <a-typography>
        <div style="display: flex; flex-direction: row;align-items: center;justify-content: space-between;">
            <a>
                <router-link to="/">Back</router-link>
            </a>
            <div style="display:flex; flex-direction: row;align-items: center; float: right; width: 200px;">
                <a-space>
                    <label>Filter</label>
                    <a-input style="width: 150px;" v-model:value="filter"></a-input>
                </a-space>
            </div>
        </div>
        <div id="canvasParent"
             style="margin-top: 70px; width: 100%; height: 100%; display: flex; align-items: center; justify-content: center;">
            <canvas ref="flameGraphCanvas" width="0" height="0"></canvas>
        </div>
        <div style="margin-top: 20px; width: 100%; display: flex; align-items: center; justify-content: center;">
            <a-space>
                <a-button type="primary" @click="sample(!isSampling)">{{
                        isSampling ? "Stop" : "Start"
                    }}
                </a-button>
                <a-button @click="resetZoom" v-if="selectedNode!=null">
                    Reset Zoom
                </a-button>
            </a-space>
        </div>
    </a-typography>
</template>

<script>
import {ref} from "vue";
import {invoke} from "@tauri-apps/api/tauri";

export default {
    data() {
        const flameGraphData = ref({});
        const filter = ref('');
        const isSampling = ref(false);
        return {
            canvasWidth: 0,
            flameGraphData,
            tooltipTitle: "",
            tooltipOpen: false,
            tooltipContent: "",
            tooltipLeft: 0,
            tooltipTop: 0,
            filter,
            interval: null,
            isSampling,
            selectedNode: null,
            clickHandler: null,
            moveHandler: null,
            leaveHandler: null,
        }
    },
    created() {
        window.onresize = () => {
            this.resizeCanvas();
            this.drawFlameGraphOnCanvas();
        }
    },
    watch: {
        filter() {
            this.resizeCanvas();
            this.drawFlameGraphOnCanvas();
        },
    },
    methods: {
        calculateNodeWidth(node) {
            return (node.num / this.totalNum) * this.canvasWidth; // Assuming a total width of 800 pixels for the flame graph
        },
        drawFlameGraph(ctx, node, x, y, totalNum, parentAlpha) {
            const nodeWidth = this.calculateNodeWidth(node);
            const nodeHeight = 24;

            let currentX = x;
            let currentY = y - nodeHeight;

            let isMatchFilter = this.filter !== '' && node.name.toLowerCase().includes(this.filter.toLowerCase());
            let calcAlpha = Math.max(0.0, 1 * Math.min(1, 2 * node.num / totalNum));
            let alpha = parentAlpha == null ? calcAlpha : Math.min(parentAlpha, calcAlpha);
            // Draw the rectangle representing the current node
            ctx.fillStyle = isMatchFilter ? "purple" : "rgba(203,27,69," + alpha + ")"; // You can customize the color as needed
            ctx.fillRect(currentX, currentY, nodeWidth, nodeHeight);
            // save coordinates for tooltip
            node.rectangle = {
                x: currentX,
                y: currentY,
                width: nodeWidth,
                height: nodeHeight
            };

            // Draw the border around the rectangle
            ctx.strokeStyle = "#eee"; // You can customize the border color as needed
            ctx.strokeRect(currentX, currentY, nodeWidth, nodeHeight);

            // Draw the function name inside the rectangle
            ctx.fillStyle = isMatchFilter || alpha >= 0.5 ? "#fff" : "#000"; // You can customize the text color as needed
            ctx.font = "12px MonoLisa Freeze"; // You can customize the font and size as needed
            let percent = (node.num / this.totalNum * 100).toFixed(2) + "%";
            let text = this.truncateText(ctx, node.name + '(' + percent + ')', nodeWidth - 10);
            let textX = currentX + this.getXCoordinateForCenteredText(ctx, text, nodeWidth);
            const textY = currentY + 16;
            ctx.fillText(text, textX, textY);

            if (node.children) {
                for (const key of Object.keys(node.children)) {
                    const childNode = node.children[key];
                    const childNodeWidth = this.calculateNodeWidth(childNode);
                    this.drawFlameGraph(ctx, childNode, currentX, currentY, node.num, alpha);
                    currentX += childNodeWidth;
                }
            }
        },
        drawFlameGraphOnCanvas() {
            this.$nextTick(() => {
                const canvas = this.$refs.flameGraphCanvas;
                let width = canvas.width;
                let height = canvas.height;
                const ctx = this.makeHighRes(canvas);
                ctx.fillStyle = "#fff";
                ctx.fillRect(0, 0, width, height);
                let flameGraphData = this.selectedNode == null ? this.flameGraphData.value : this.selectedNode;
                this.totalNum = flameGraphData.num;
                this.drawFlameGraph(ctx, flameGraphData, 0, height, 0, null);
                canvas.removeEventListener('mousemove', this.moveHandler);
                canvas.removeEventListener('click', this.clickHandler);
                canvas.removeEventListener('mouseleave', this.leaveHandler);
                this.moveHandler = (event) => {
                    const rect = canvas.getBoundingClientRect();
                    const x = event.clientX - rect.left;
                    const y = event.clientY - rect.top;

                    const node = this.findNodeAtCoordinates(flameGraphData, x, y);
                    if (node) {
                        this.showTooltip(canvas, node);
                    } else {
                        this.hideTooltip();
                    }
                };
                canvas.addEventListener('mousemove', this.moveHandler);
                this.clickHandler = (event) => {
                    const rect = canvas.getBoundingClientRect();
                    const x = event.clientX - rect.left;
                    const y = event.clientY - rect.top;

                    const node = this.findNodeAtCoordinates(flameGraphData, x, y);
                    if (node) {
                        this.selectedNode = node;
                        this.resizeCanvas();
                        this.drawFlameGraphOnCanvas();
                    }
                };
                this.leaveHandler = () => {
                    this.hideTooltip();
                };
                canvas.addEventListener('click', this.clickHandler);
                canvas.addEventListener('mouseleave', this.leaveHandler);
            });
        },
        findNodeAtCoordinates(node, x, y) {
            if (x < node.rectangle.x || x > node.rectangle.x + node.rectangle.width ||
                y < node.rectangle.y || y > node.rectangle.y + node.rectangle.height) {
                for (const key of Object.keys(node.children)) {
                    const childNode = node.children[key];
                    const foundNode = this.findNodeAtCoordinates(childNode, x, y);
                    if (foundNode) {
                        return foundNode;
                    }
                }
            } else {
                return node;
            }
        },
        showTooltip(canvas, node) {
            console.log(node.rectangle);

            const canvasRect = canvas.getBoundingClientRect();
            const scrollLeft = document.documentElement.scrollLeft;
            const scrollTop = document.documentElement.scrollTop;

            console.log(canvasRect);

            this.tooltipLeft = canvasRect.left + scrollLeft + node.rectangle.x - 20;
            this.tooltipTop = canvasRect.top + scrollTop + node.rectangle.y - node.rectangle.height;

            this.tooltipTitle = `${node.name}`;
            this.tooltipOpen = true;
            this.tooltipContent = `Num: ${node.num} (${(node.num / this.totalNum * 100).toFixed(2)}%)`;
        },
        hideTooltip() {
            this.tooltipOpen = false;
        },
        makeHighRes(canvas) {
            const ctx = canvas.getContext('2d');
            // Get the device pixel ratio, falling back to 1.
            const dpr = window.devicePixelRatio || window.webkitDevicePixelRatio || window.mozDevicePixelRatio || 1;

            // Get the size of the canvas in CSS pixels.
            const oldWidth = canvas.width;
            const oldHeight = canvas.height;
            // Give the canvas pixel dimensions of their CSS
            // size * the device pixel ratio.
            canvas.width = Math.round(oldWidth * dpr);
            canvas.height = Math.round(oldHeight * dpr);
            canvas.style.width = oldWidth + 'px';
            canvas.style.height = oldHeight + 'px';
            // Scale all drawing operations by the dpr, so you
            // don't have to worry about the difference.
            ctx.scale(dpr, dpr);
            return ctx;
        },
        truncateText(ctx, text, maxWidth) {
            let truncatedText = text;
            const textWidth = ctx.measureText(text).width;
            if (textWidth > maxWidth) {
                // Find the index where the text needs to be truncated
                let i = 0;
                while (ctx.measureText(truncatedText.substring(0, i))
                    .width <= maxWidth && i < truncatedText.length) {
                    i++;
                }
                // Truncate the text and add "..." at the end
                truncatedText = truncatedText.substring(0, i - 1);
            }
            return truncatedText;
        },
        // Function to get the X-coordinate for centering the text
        getXCoordinateForCenteredText(ctx, text, rectWidth) {
            const textWidth = ctx.measureText(text).width;
            return rectWidth / 2 - textWidth / 2;
        },
        resizeCanvas() {
            const canvas = this.$refs.flameGraphCanvas;
            // 最长调用链 * 24
            canvas.height = this.getMaxDepth() * 24;
            canvas.width = canvas.parentElement.clientWidth * 0.9;
            this.canvasWidth = canvas.width;
        },
        getMaxDepth() {
            let maxDepth = 0;
            const traverse = (node, depth) => {
                if (depth > maxDepth) {
                    maxDepth = depth;
                }
                if (node.children) {
                    for (const key of Object.keys(node.children)) {
                        const childNode = node.children[key];
                        traverse(childNode, depth + 1);
                    }
                }
            };
            traverse(this.flameGraphData.value, 0);
            return maxDepth;
        },
        async getJstackInfo() {
            this.flameGraphData.value = await invoke("get_jstack_info", {pid: this.$route.params.pid});
            this.resizeCanvas();
            this.drawFlameGraphOnCanvas();
        },
        async clearJstackInfo() {
            await invoke("clear_jstack_info", {});
        },
        async sample(sample) {
            if (sample) {
                this.isSampling = true;
                this.interval = setInterval(
                    async () => {
                        await this.getJstackInfo();
                    }
                    , 1000);
            } else {
                clearInterval(this.interval);
                this.isSampling = false;
                await this.clearJstackInfo();
            }
        },
        resetZoom() {
            this.selectedNode = null;
            this.resizeCanvas();
            this.drawFlameGraphOnCanvas();
        }
    },
    async mounted() {
        this.resizeCanvas();
        await this.clearJstackInfo();
    }
}
</script>