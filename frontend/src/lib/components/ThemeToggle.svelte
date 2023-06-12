<script lang="ts">
    import IconLight from "virtual:icons/lucide/sun";
    import IconDark from "virtual:icons/lucide/moon";
    
    function handle() {
        const storage = localStorage.getItem('theme')
        if (storage == 'light') {
            localStorage.setItem('theme' ,'dark')
            document.documentElement.classList.add('dark')
            document.documentElement.classList.remove('light')
        } else if (storage == 'dark') {
            localStorage.setItem('theme' ,'light')
            document.documentElement.classList.remove('dark')
        } else if (!storage) {
            document.documentElement.classList.toggle('dark')
            console.log('No storage')
            if (document.documentElement.classList.contains('dark')) {
                localStorage.setItem('theme' ,'dark')
            } else {
                localStorage.setItem('theme' ,'light')
            }
        } else {
            localStorage.setItem('theme','light');
            handle()
        }
    }
</script>

<button class="button-transparent" on:click={handle}>
    <IconLight class="dark:hidden" />
    <IconDark class="hidden dark:inline" />
</button>
