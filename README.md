# 功能

使用bangumi_API搜索番剧信息，并重命名。

# 方法

in_folder:  番剧所在文件夹路径
```
├─in_folder  
|   ├─...  
|   ├─...  
|   ├─video.mp4(avi,flv,mkv)   
|   ├─video.sc.ass(str)
|   ├─video.tc.ass(str)
|   ├─video.ass(str)
|   ├─...
```
out_folder: 媒体库所在文件夹路径
```
├─out_folder    
|   ├─{番剧名 （年份）}  
|   ├─...  
|   ├─ep01 - title.mp4(avi,flv,mkv)     
|   ├─ep01 - title.sc.ass(str)
|   ├─ep01 - title.tc.ass(str) 
|   ├─ep01 - title.ass(str)
|   ├─...
```

# 特点

1. 创建{番剧名 （年份）}格式的文件夹。
2. 匹配乱序文件名。
3. 匹配特殊字符。
4. 匹配简、繁两种字幕同时存在的情况。
5. 视频文件硬链接，字幕文件直接复制，不影响做种，不破坏数据，方便管理。
6. 根据番剧集名重命名。

# 依赖库

```
pip install requests
```

# TODO

- [ ] GUI界面
- [ ] rust重构

# License

Licensed under the [MIT](LICENSE) license.