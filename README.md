# 功能

使用bangumi_API搜索番剧信息，并重命名。

# 方法
in_folder: 输入番剧所在文件夹路径

├─in_folder  
|&emsp;&emsp;├─...  
|&emsp;&emsp;├─...  
|&emsp;&emsp;├─video.mp4    
|&emsp;&emsp;├─video.ass  
|&emsp;&emsp;├─...

out_folder: 输入媒体库所在文件夹路径

├─out_folder    
|&emsp;&emsp;├─{番剧名 （年份）}  
|&emsp;&emsp;&emsp;&emsp;├─...  
|&emsp;&emsp;&emsp;&emsp;├─ep01 - title.mp4  
|&emsp;&emsp;&emsp;&emsp;├─ep01 - title.ass  
|&emsp;&emsp;&emsp;&emsp;├─...

# 特点

1. 创建{番剧名 （年份）}格式的文件夹。
2. 匹配乱序文件名。
3. 匹配特殊字符。
4. 视频文件硬链接，字幕文件直接复制，不影响做种，不破坏数据，方便管理。
5. 根据番剧集名重命名。

# 依赖库

```
pip install requests
```

# TODO

- [ ] GUI界面
- [ ] rust重构